use crate::common::drive_file;
use crate::common::file_tree_drive;
use crate::common::file_tree_drive::FileTreeDrive;
use crate::common::hub_helper;
use crate::common::md5_writer::Md5Writer;
use crate::common::parse_md5_digest;
use crate::files;
use crate::hub::Hub;
use async_recursion::async_recursion;
use bytesize::ByteSize;
use error_trace::ErrorTrace;
use futures::stream::StreamExt;
use google_drive3::hyper;
use md5::Digest;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;

pub struct Config {
    pub file_id: String,
    pub existing_file_action: ExistingFileAction,
    pub follow_shortcuts: bool,
    pub download_directories: bool,
    pub destination: Destination,
}

impl Config {
    fn canonical_destination_root(&self) -> Result<PathBuf, Error> {
        match &self.destination {
            Destination::CurrentDir => {
                let current_path = PathBuf::from(".");
                let canonical_current_path = current_path
                    .canonicalize()
                    .map_err(|err| Error::CanonicalizeDestinationPath(current_path.clone(), err))?;
                Ok(canonical_current_path)
            }

            Destination::Path(path) => {
                if !path.exists() {
                    Err(Error::DestinationPathDoesNotExist(path.clone()))
                } else if !path.is_dir() {
                    Err(Error::DestinationPathNotADirectory(path.clone()))
                } else {
                    path.canonicalize()
                        .map_err(|err| Error::CanonicalizeDestinationPath(path.clone(), err))
                }
            }

            Destination::Stdout => Err(Error::StdoutNotValidDestination),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Destination {
    CurrentDir,
    Path(PathBuf),
    Stdout,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExistingFileAction {
    Abort,
    Overwrite,
}

#[async_recursion]
pub async fn download(config: Config) -> Result<(), Error> {
    let hub = hub_helper::get_hub().await.map_err(Error::Hub)?;

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    err_if_file_exists(&file, &config)?;
    err_if_directory(&file, &config)?;
    err_if_shortcut(&file, &config)?;

    if drive_file::is_shortcut(&file) {
        let target_file_id = file.shortcut_details.and_then(|details| details.target_id);

        let file_id = target_file_id.ok_or(Error::MissingShortcutTarget)?;

        download(Config { file_id, ..config }).await?;
    } else if drive_file::is_directory(&file) {
        download_directory(&hub, &file, &config).await?;
    } else {
        download_regular(&hub, &file, &config).await?;
    }

    Ok(())
}

pub async fn download_regular(
    hub: &Hub,
    file: &google_drive3::api::File,
    config: &Config,
) -> Result<(), Error> {
    let body = download_file(hub, &config.file_id)
        .await
        .map_err(|err| Error::DownloadFile(Box::new(err)))?;

    if config.destination == Destination::Stdout {
        save_body_to_stdout(body).await?;
    } else {
        let file_name = file.name.clone().ok_or(Error::MissingFileName)?;
        let root_path = config.canonical_destination_root()?;
        let abs_file_path = root_path.join(&file_name);

        println!("Downloading {file_name}");
        let md5_checksum = file.md5_checksum.as_deref().and_then(parse_md5_digest);
        save_body_to_file(body, &abs_file_path, md5_checksum.as_ref()).await?;
        println!("Successfully downloaded {file_name}");
    }

    Ok(())
}

pub async fn download_directory(
    hub: &Hub,
    file: &google_drive3::api::File,
    config: &Config,
) -> Result<(), Error> {
    let tree = FileTreeDrive::from_file(hub, file)
        .await
        .map_err(Error::CreateFileTree)?;

    let tree_info = tree.info();

    println!(
        "Found {} files in {} directories with a total size of {}",
        tree_info.file_count,
        tree_info.folder_count,
        ByteSize::b(tree_info.total_file_size).display().si(),
    );

    let root_path = config.canonical_destination_root()?;

    for folder in &tree.folders() {
        let folder_path = folder.relative_path();
        let abs_folder_path = root_path.join(&folder_path);

        println!("Creating directory {}", folder_path.display());
        fs::create_dir_all(&abs_folder_path)
            .map_err(|err| Error::CreateDirectory(abs_folder_path, err))?;

        for file in folder.files() {
            let file_path = file.relative_path();
            let abs_file_path = root_path.join(&file_path);

            if local_file_is_identical(&abs_file_path, &file).await {
                continue;
            }

            let body = download_file(hub, &file.drive_id)
                .await
                .map_err(|err| Error::DownloadFile(Box::new(err)))?;

            println!("Downloading file '{}'", file_path.display());
            save_body_to_file(body, &abs_file_path, file.md5.as_ref()).await?;
        }
    }

    println!(
        "Downloaded {} files in {} directories with a total size of {}",
        tree_info.file_count,
        tree_info.folder_count,
        ByteSize::b(tree_info.total_file_size).display().si()
    );

    Ok(())
}

pub async fn download_file(hub: &Hub, file_id: &str) -> Result<hyper::Body, google_drive3::Error> {
    let (response, _) = hub
        .files()
        .get(file_id)
        .supports_all_drives(true)
        .param("alt", "media")
        .add_scope(google_drive3::api::Scope::Full)
        .doit()
        .await?;

    Ok(response.into_body())
}

#[derive(Debug)]
pub enum Error {
    Hub(hub_helper::Error),
    GetFile(Box<google_drive3::Error>),
    DownloadFile(Box<google_drive3::Error>),
    MissingFileName,
    FileExists(PathBuf),
    IsDirectory(String),
    Md5Mismatch {
        expected: Option<Digest>,
        actual: Digest,
    },
    CreateFile(io::Error),
    CreateDirectory(PathBuf, io::Error),
    CopyFile(io::Error),
    RenameFile(io::Error),
    ReadChunk(hyper::Error),
    WriteChunk(io::Error),
    CreateFileTree(file_tree_drive::Error),
    DestinationPathDoesNotExist(PathBuf),
    DestinationPathNotADirectory(PathBuf),
    CanonicalizeDestinationPath(PathBuf, io::Error),
    MissingShortcutTarget,
    IsShortcut(String),
    StdoutNotValidDestination,
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(err) => write!(f, "{err}"),
            Error::GetFile(err) => write!(f, "Failed getting file: {err}"),
            Error::DownloadFile(err) => write!(f, "Failed to download file: {err}"),
            Error::MissingFileName => write!(f, "File does not have a name"),
            Error::FileExists(path) => write!(
                f,
                "File '{}' already exists, use --overwrite to overwrite it",
                path.display()
            ),
            Error::IsDirectory(name) => write!(
                f,
                "'{name}' is a directory, use --recursive to download directories"
            ),
            Error::Md5Mismatch { expected, actual } => {
                write!(
                    f,
                    "MD5 mismatch, expected: {expected:x?}, actual: {actual:x?}"
                )
            }
            Error::CreateFile(err) => write!(f, "Failed to create file: {err}"),
            Error::CreateDirectory(path, err) => write!(
                f,
                "Failed to create directory '{}': {}",
                path.display(),
                err
            ),
            Error::CopyFile(err) => write!(f, "Failed to copy file: {err}"),
            Error::RenameFile(err) => write!(f, "Failed to rename file: {err}"),
            Error::ReadChunk(err) => write!(f, "Failed read from stream: {err}"),
            Error::WriteChunk(err) => write!(f, "Failed write to file: {err}"),
            Error::CreateFileTree(err) => write!(f, "Failed to create file tree: {err}"),
            Error::DestinationPathDoesNotExist(path) => {
                write!(f, "Destination path '{}' does not exist", path.display())
            }
            Error::DestinationPathNotADirectory(path) => {
                write!(
                    f,
                    "Destination path '{}' is not a directory",
                    path.display()
                )
            }
            Error::CanonicalizeDestinationPath(path, err) => write!(
                f,
                "Failed to canonicalize destination path '{}': {}",
                path.display(),
                err
            ),
            Error::MissingShortcutTarget => write!(f, "Shortcut does not have a target"),
            Error::IsShortcut(name) => write!(
                f,
                "'{name}' is a shortcut, use --follow-shortcuts to download the file it points to"
            ),
            Error::StdoutNotValidDestination => write!(
                f,
                "Stdout is not a valid destination for this combination of options"
            ),
        }
    }
}

// TODO: move to common
pub async fn save_body_to_file(
    mut body: hyper::Body,
    file_path: &PathBuf,
    expected_md5: Option<&Digest>,
) -> Result<(), Error> {
    // Create temporary file
    let tmp_file_path = file_path.with_extension("incomplete");
    let file = File::create(&tmp_file_path)
        .await
        .map_err(Error::CreateFile)?;

    // Wrap file in writer that calculates md5
    let mut writer = Md5Writer::new(file);

    // Read chunks from stream and write to file
    while let Some(chunk_result) = body.next().await {
        let chunk = chunk_result.map_err(Error::ReadChunk)?;
        writer.write_all(&chunk).await.map_err(Error::WriteChunk)?;
    }

    // Check md5
    err_if_md5_mismatch(expected_md5, &writer.md5())?;

    // Rename temporary file to final file
    fs::rename(&tmp_file_path, file_path).map_err(Error::RenameFile)
}

// TODO: move to common
pub async fn save_body_to_stdout(mut body: hyper::Body) -> Result<(), Error> {
    let mut stdout = io::stdout();

    // Read chunks from stream and write to stdout
    while let Some(chunk_result) = body.next().await {
        let chunk = chunk_result.map_err(Error::ReadChunk)?;
        stdout.write_all(&chunk).map_err(Error::WriteChunk)?;
    }

    Ok(())
}

fn err_if_file_exists(file: &google_drive3::api::File, config: &Config) -> Result<(), Error> {
    let file_name = file.name.clone().ok_or(Error::MissingFileName)?;

    let file_path = match &config.destination {
        Destination::CurrentDir => Some(PathBuf::from(".").join(file_name)),
        Destination::Path(path) => Some(path.join(file_name)),
        Destination::Stdout => None,
    };

    match file_path {
        Some(path) => {
            if path.exists() && config.existing_file_action == ExistingFileAction::Abort {
                Err(Error::FileExists(path.clone()))
            } else {
                Ok(())
            }
        }

        None => Ok(()),
    }
}

fn err_if_directory(file: &google_drive3::api::File, config: &Config) -> Result<(), Error> {
    if drive_file::is_directory(file) && !config.download_directories {
        let name = file
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        Err(Error::IsDirectory(name))
    } else {
        Ok(())
    }
}

fn err_if_shortcut(file: &google_drive3::api::File, config: &Config) -> Result<(), Error> {
    if drive_file::is_shortcut(file) && !config.follow_shortcuts {
        let name = file
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        Err(Error::IsShortcut(name))
    } else {
        Ok(())
    }
}

fn err_if_md5_mismatch(expected: Option<&Digest>, actual: &Digest) -> Result<(), Error> {
    let is_matching = expected.as_ref().is_none_or(|md5| md5 == &actual);

    if is_matching {
        Ok(())
    } else {
        Err(Error::Md5Mismatch {
            expected: expected.copied(),
            actual: *actual,
        })
    }
}

async fn local_file_is_identical(path: &PathBuf, file: &file_tree_drive::File) -> bool {
    if path.exists() {
        match compute_md5_from_path(path).await {
            Ok(file_md5) => file.md5.as_ref().is_some_and(|md5| md5 == &file_md5),
            Err(err) => {
                eprintln!(
                    "Warning: Error while computing md5 of '{}': {}",
                    path.display(),
                    err.trace(),
                );
                false
            }
        }
    } else {
        false
    }
}

async fn compute_md5_from_path(path: &PathBuf) -> Result<Digest, io::Error> {
    let input = File::open(path).await?;
    let reader = BufReader::new(input);
    compute_md5_from_async_reader(reader).await
}

async fn compute_md5_from_async_reader<R>(mut reader: R) -> Result<Digest, io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut context = md5::Context::new();
    let mut buffer = [0; 4096];

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => context.consume(&buffer[..count]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }

    Ok(context.compute())
}
