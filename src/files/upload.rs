use std::{
    error,
    fmt::{Display, Formatter},
    fs, io,
    ops::Not,
    path::{Path, PathBuf},
    time::Duration,
};

use bytesize::ByteSize;
use mime::Mime;

use crate::{
    common::{
        delegate::{BackoffConfig, ChunkSize, UploadDelegate, UploadDelegateConfig},
        file_helper,
        file_info::{self, FileInfo},
        file_tree::{self, FileTree},
        hub_helper::{get_hub, GetHubError},
        id_gen::IdGen,
        FileTreeLike, FolderLike,
    },
    files::{self, info::DisplayConfig, mkdir, FileResult},
    hub::Hub,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "they are orthogonal one each other"
)]
pub(crate) struct Config {
    pub(crate) file_path: Option<PathBuf>,
    pub(crate) mime_type: Option<Mime>,
    pub(crate) parents: Option<Vec<String>>,
    pub(crate) chunk_size: ChunkSize,
    pub(crate) print_chunk_errors: bool,
    pub(crate) print_chunk_info: bool,
    pub(crate) upload_directories: bool,
    pub(crate) print_only_id: bool,
}

impl Config {
    fn split(self) -> (CommonConfig, Option<PathBuf>, UploadDelegateConfig) {
        let Self {
            file_path,
            mime_type,
            parents,
            chunk_size,
            print_chunk_errors,
            print_chunk_info,
            upload_directories,
            print_only_id,
        } = self;
        (
            CommonConfig {
                mime_type,
                parents,
                upload_directories,
                print_only_id,
            },
            file_path,
            UploadDelegateConfig {
                chunk_size,
                backoff_config: BackoffConfig {
                    max_retries: 100_000,
                    min_sleep: Duration::from_secs(1),
                    max_sleep: Duration::from_secs(60),
                },
                print_chunk_errors,
                print_chunk_info,
            },
        )
    }
}

pub(crate) async fn upload(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;

    let (common, file_path, delegate_config) = config.split();

    if let Some(file_path) = file_path {
        if file_path.is_dir() && common.upload_directories.not() {
            return Err(Error::IsDirectory(file_path));
        }

        if file_path.is_dir() {
            upload_directory(
                &hub,
                UploadDirectoryConfig { file_path, common },
                &delegate_config,
            )
            .await?;
        } else {
            upload_regular(
                &hub,
                UploadRegularConfig {
                    file_path: FilePath::Regular(file_path),
                    common,
                },
                &delegate_config,
            )
            .await?;
        }
    } else {
        let tmp_file = file_helper::stdin_to_file().map_err(Error::StdinToFile)?;

        upload_regular(
            &hub,
            UploadRegularConfig {
                file_path: FilePath::Temp(tmp_file),
                common,
            },
            &delegate_config,
        )
        .await?;
    }

    Ok(())
}

#[derive(Debug)]
enum FilePath {
    Regular(PathBuf),
    Temp(mktemp::Temp),
}

impl FilePath {
    fn as_path(&self) -> &Path {
        match self {
            FilePath::Regular(path_buf) => path_buf.as_path(),
            FilePath::Temp(temp) => temp.as_path(),
        }
    }

    fn into_path_buf(self) -> PathBuf {
        match self {
            FilePath::Regular(path_buf) => path_buf,
            FilePath::Temp(temp) => temp.release(),
        }
    }
}

#[derive(Debug)]
struct CommonConfig {
    mime_type: Option<Mime>,
    parents: Option<Vec<String>>,
    upload_directories: bool,
    print_only_id: bool,
}

#[derive(Debug)]
struct UploadRegularConfig {
    file_path: FilePath,
    common: CommonConfig,
}

async fn upload_regular(
    hub: &Hub,
    config: UploadRegularConfig,
    delegate_config: &UploadDelegateConfig,
) -> Result<(), Error> {
    let file = match fs::File::open(config.file_path.as_path()) {
        Ok(file) => file,
        Err(err) => {
            return Err(Error::OpenFile(config.file_path.into_path_buf(), err));
        }
    };

    let file_info = match FileInfo::from_file(
        &file,
        file_info::Config {
            file_path: config.file_path.as_path(),
            mime_type: config.common.mime_type.as_ref(),
            parents: config.common.parents,
        },
    ) {
        Ok(file_info) => file_info,
        Err(source) => {
            return Err(Error::FileInfo {
                path: config.file_path.into_path_buf(),
                source,
            })
        }
    };

    let reader = std::io::BufReader::new(file);

    if !config.common.print_only_id {
        println!("Uploading {}", config.file_path.as_path().display());
    }

    let file = upload_file(hub, reader, None, file_info, delegate_config)
        .await
        .map_err(|err| Error::Upload(Box::new(err)))?;

    if config.common.print_only_id {
        print!("{}", file.id.unwrap_or_default());
    } else {
        println!("File successfully uploaded");
        files::info::print_file_info(&file, &DisplayConfig::default());
    }

    Ok(())
}

#[derive(Debug)]
struct UploadDirectoryConfig {
    file_path: PathBuf,
    common: CommonConfig,
}

async fn upload_directory(
    hub: &Hub,
    config: UploadDirectoryConfig,
    delegate_config: &UploadDelegateConfig,
) -> Result<(), Error> {
    let mut ids = IdGen::new(hub, delegate_config);
    let tree = FileTree::from_path(&config.file_path, &mut ids)
        .await
        .map_err(Error::CreateFileTree)?;

    let tree_info = tree.info();

    if !config.common.print_only_id {
        println!(
            "Found {} files in {} directories with a total size of {}",
            tree_info.file_count,
            tree_info.folder_count,
            ByteSize::b(tree_info.total_file_size).display().si(),
        );
    }

    for folder in &tree.folders() {
        let folder_parents = folder
            .info
            .parent
            .as_ref()
            .map(|p| vec![p.drive_id.clone()])
            .or_else(|| config.common.parents.clone());

        if !config.common.print_only_id {
            println!(
                "Creating directory '{}' with id: {}",
                folder.relative_path().display(),
                folder.info.drive_id
            );
        }

        let drive_folder = mkdir::create_directory(
            hub,
            &mkdir::Config {
                id: Some(folder.info.drive_id.clone()),
                name: folder.info.name.clone(),
                parents: folder_parents,
                print_only_id: false,
            },
            delegate_config,
        )
        .await
        .map_err(|err| Error::Mkdir(Box::new(err)))?;

        if config.common.print_only_id {
            println!(
                "{}: {}",
                folder.relative_path().display(),
                folder.info.drive_id
            );
        }

        let folder_id = drive_folder.id.ok_or(Error::DriveFolderMissingId)?;
        let parents = Some(vec![folder_id.clone()]);

        for file in folder.files() {
            let os_file = match fs::File::open(&file.path) {
                Ok(os_file) => os_file,
                Err(err) => {
                    return Err(Error::OpenFile(config.file_path, err));
                }
            };

            let file_info = file.info(parents.clone());

            if !config.common.print_only_id {
                println!(
                    "Uploading file '{}' with id: {}",
                    file.relative_path().display(),
                    file.drive_id
                );
            }

            upload_file(
                hub,
                os_file,
                Some(file.drive_id.clone()),
                file_info,
                delegate_config,
            )
            .await
            .map_err(|err| Error::Upload(Box::new(err)))?;

            if config.common.print_only_id {
                println!("{}: {}", file.relative_path().display(), file.drive_id);
            }
        }
    }

    if !config.common.print_only_id {
        println!(
            "Uploaded {} files in {} directories with a total size of {}",
            tree_info.file_count,
            tree_info.folder_count,
            ByteSize::b(tree_info.total_file_size).display().si(),
        );
    }

    Ok(())
}

#[expect(
    clippy::result_large_err,
    reason = "Ok variant is bigger, see test next to FileResult"
)]
pub(crate) async fn upload_file<RS>(
    hub: &Hub,
    src_file: RS,
    file_id: Option<String>,
    file_info: FileInfo<'_>,
    delegate_config: &UploadDelegateConfig,
) -> FileResult
where
    RS: google_drive3::client::ReadSeek,
{
    let dst_file = google_drive3::api::File {
        id: file_id,
        name: Some(file_info.name.into_owned()),
        mime_type: Some(file_info.mime_type.to_string()),
        parents: file_info.parents,
        ..google_drive3::api::File::default()
    };

    let chunk_size_bytes = delegate_config.chunk_size.in_bytes();
    let mut delegate = UploadDelegate::new(delegate_config);

    let req = hub
        .files()
        .create(dst_file)
        .param("fields", "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,description,webContentLink,webViewLink")
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true);

    let (_, file) = if file_info.size > chunk_size_bytes {
        req.upload_resumable(src_file, file_info.mime_type.into_owned())
            .await?
    } else {
        req.upload(src_file, file_info.mime_type.into_owned())
            .await?
    };

    Ok(file)
}

#[derive(Debug)]
pub(crate) enum Error {
    Hub(GetHubError),
    FileInfo {
        path: PathBuf,
        source: file_info::FromFileError,
    },
    OpenFile(PathBuf, io::Error),
    StdinToFile(file_helper::StdinToFileError),
    Upload(Box<google_drive3::Error>),
    IsDirectory(PathBuf),
    DriveFolderMissingId,
    CreateFileTree(file_tree::errors::FileTree),
    Mkdir(Box<google_drive3::Error>),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::FileInfo { source, .. } => Some(source),
            Error::OpenFile(_, source) => Some(source),
            Error::StdinToFile(source) => Some(source),
            Error::Upload(source) | Error::Mkdir(source) => Some(source),
            Error::IsDirectory(_) | Error::DriveFolderMissingId => None,
            Error::CreateFileTree(file_tree) => Some(file_tree),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::FileInfo { path, source: _ } => {
                write!(f, "unable to get file info for '{}'", path.display())
            }
            Error::OpenFile(path, _) => {
                write!(f, "unable to open file '{}'", path.display())
            }
            Error::StdinToFile(_) => f.write_str("unable to write stdin to file"),
            Error::Upload(_) => f.write_str("unable to upload file"),
            Error::IsDirectory(path) => write!(
                f,
                "'{}' is a directory, use --recursive to upload directories",
                path.display()
            ),
            Error::DriveFolderMissingId => {
                f.write_str("folder created on drive does not have an id")
            }
            Error::CreateFileTree(_) => f.write_str("unable to create file tree"),
            Error::Mkdir(_) => f.write_str("unable to create directory"),
        }
    }
}
