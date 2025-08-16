use crate::common::delegate::BackoffConfig;
use crate::common::delegate::ChunkSize;
use crate::common::delegate::UploadDelegate;
use crate::common::delegate::UploadDelegateConfig;
use crate::common::file_helper;
use crate::common::file_info;
use crate::common::file_info::FileInfo;
use crate::common::file_tree;
use crate::common::file_tree::FileTree;
use crate::common::hub_helper;
use crate::common::id_gen::IdGen;
use crate::files;
use crate::files::info::DisplayConfig;
use crate::files::mkdir;
use crate::hub::Hub;
use bytesize::ByteSize;
use mime::Mime;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

#[expect(
    clippy::struct_excessive_bools,
    reason = "they are orthogonal one each other"
)]
pub struct Config {
    pub file_path: Option<PathBuf>,
    pub mime_type: Option<Mime>,
    pub parents: Option<Vec<String>>,
    pub chunk_size: ChunkSize,
    pub print_chunk_errors: bool,
    pub print_chunk_info: bool,
    pub upload_directories: bool,
    pub print_only_id: bool,
}

pub async fn upload(config: Config) -> Result<(), Error> {
    let hub = hub_helper::get_hub().await.map_err(Error::Hub)?;

    let delegate_config = UploadDelegateConfig {
        chunk_size: config.chunk_size.clone(),
        backoff_config: BackoffConfig {
            max_retries: 100_000,
            min_sleep: Duration::from_secs(1),
            max_sleep: Duration::from_secs(60),
        },
        print_chunk_errors: config.print_chunk_errors,
        print_chunk_info: config.print_chunk_info,
    };

    if let Some(path) = &config.file_path {
        err_if_directory(path, &config)?;

        if path.is_dir() {
            upload_directory(&hub, &config, delegate_config).await?;
        } else {
            upload_regular(&hub, &config, delegate_config).await?;
        }
    } else {
        let tmp_file = file_helper::stdin_to_file().map_err(Error::StdinToFile)?;

        upload_regular(
            &hub,
            &Config {
                file_path: Some(tmp_file.as_ref().to_path_buf()),
                ..config
            },
            delegate_config,
        )
        .await?;
    }

    Ok(())
}

pub async fn upload_regular(
    hub: &Hub,
    config: &Config,
    delegate_config: UploadDelegateConfig,
) -> Result<(), Error> {
    let file_path = config.file_path.as_ref().unwrap();
    let file = fs::File::open(file_path).map_err(|err| Error::OpenFile(file_path.clone(), err))?;

    let file_info = match FileInfo::from_file(
        &file,
        file_info::Config {
            file_path,
            mime_type: config.mime_type.as_ref(),
            parents: config.parents.clone(),
        },
    ) {
        Ok(file_info) => file_info,
        Err(source) => {
            return Err(Error::FileInfo {
                path: file_path.clone(),
                source,
            })
        }
    };

    let reader = std::io::BufReader::new(file);

    if !config.print_only_id {
        println!("Uploading {}", file_path.display());
    }

    let file = upload_file(hub, reader, None, file_info, delegate_config)
        .await
        .map_err(|err| Error::Upload(Box::new(err)))?;

    if config.print_only_id {
        print!("{}", file.id.unwrap_or_default());
    } else {
        println!("File successfully uploaded");
        let fields = files::info::prepare_fields(&file, &DisplayConfig::default());
        files::info::print_fields(&fields);
    }

    Ok(())
}

pub async fn upload_directory(
    hub: &Hub,
    config: &Config,
    delegate_config: UploadDelegateConfig,
) -> Result<(), Error> {
    let mut ids = IdGen::new(hub, &delegate_config);
    let tree = FileTree::from_path(config.file_path.as_ref().unwrap(), &mut ids)
        .await
        .map_err(Error::CreateFileTree)?;

    let tree_info = tree.info();

    if !config.print_only_id {
        println!(
            "Found {} files in {} directories with a total size of {}",
            tree_info.file_count,
            tree_info.folder_count,
            ByteSize::b(tree_info.total_file_size).display().si(),
        );
    }

    for folder in &tree.folders() {
        let folder_parents = folder
            .parent
            .as_ref()
            .map(|p| vec![p.drive_id.clone()])
            .or_else(|| config.parents.clone());

        if !config.print_only_id {
            println!(
                "Creating directory '{}' with id: {}",
                folder.relative_path().display(),
                folder.drive_id
            );
        }

        let drive_folder = mkdir::create_directory(
            hub,
            &mkdir::Config {
                id: Some(folder.drive_id.clone()),
                name: folder.name.clone(),
                parents: folder_parents,
                print_only_id: false,
            },
            delegate_config.clone(),
        )
        .await
        .map_err(|err| Error::Mkdir(Box::new(err)))?;

        if config.print_only_id {
            println!("{}: {}", folder.relative_path().display(), folder.drive_id);
        }

        let folder_id = drive_folder.id.ok_or(Error::DriveFolderMissingId)?;
        let parents = Some(vec![folder_id.clone()]);

        for file in folder.files() {
            let os_file = fs::File::open(&file.path)
                .map_err(|err| Error::OpenFile(config.file_path.as_ref().unwrap().clone(), err))?;

            let file_info = file.info(parents.clone());

            if !config.print_only_id {
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
                delegate_config.clone(),
            )
            .await
            .map_err(|err| Error::Upload(Box::new(err)))?;

            if config.print_only_id {
                println!("{}: {}", file.relative_path().display(), file.drive_id);
            }
        }
    }

    if !config.print_only_id {
        println!(
            "Uploaded {} files in {} directories with a total size of {}",
            tree_info.file_count,
            tree_info.folder_count,
            ByteSize::b(tree_info.total_file_size).display().si(),
        );
    }

    Ok(())
}

pub async fn upload_file<RS>(
    hub: &Hub,
    src_file: RS,
    file_id: Option<String>,
    file_info: FileInfo<'_>,
    delegate_config: UploadDelegateConfig,
) -> Result<google_drive3::api::File, google_drive3::Error>
where
    RS: google_drive3::client::ReadSeek,
{
    let dst_file = google_drive3::api::File {
        id: file_id,
        name: Some(file_info.name),
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
pub enum Error {
    Hub(hub_helper::Error),
    FileInfo {
        path: PathBuf,
        source: file_info::FromFileError,
    },
    OpenFile(PathBuf, io::Error),
    StdinToFile(file_helper::StdinToFileError),
    Upload(Box<google_drive3::Error>),
    IsDirectory(PathBuf),
    DriveFolderMissingId,
    CreateFileTree(file_tree::Error),
    Mkdir(Box<google_drive3::Error>),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::FileInfo { source, .. } => Some(source),
            Error::StdinToFile(source) => Some(source),
            // FIXME: correctly impl std::error::Error
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(err) => write!(f, "{err}"),
            Error::FileInfo { path, source: _ } => {
                write!(f, "unable to get file info for '{}'", path.display())
            }
            Error::OpenFile(path, err) => {
                write!(f, "Failed to open file '{}': {}", path.display(), err)
            }
            Error::StdinToFile(_) => f.write_str("unable to write stdin to file"),
            Error::Upload(err) => write!(f, "Failed to upload file: {err}"),
            Error::IsDirectory(path) => write!(
                f,
                "'{}' is a directory, use --recursive to upload directories",
                path.display()
            ),
            Error::DriveFolderMissingId => write!(f, "Folder created on drive does not have an id"),
            Error::CreateFileTree(err) => write!(f, "Failed to create file tree: {err}"),
            Error::Mkdir(err) => write!(f, "Failed to create directory: {err}"),
        }
    }
}

fn err_if_directory(path: &Path, config: &Config) -> Result<(), Error> {
    if path.is_dir() && !config.upload_directories {
        Err(Error::IsDirectory(path.to_owned()))
    } else {
        Ok(())
    }
}
