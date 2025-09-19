pub(crate) mod errors;

use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use bytesize::ByteSize;
use error_trace::ErrorTrace;
use futures::stream::StreamExt;
use google_drive3::hyper;
use md5::Digest;
use tokio::{
    fs::{self, File},
    io::{self, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
};

use crate::{
    common::{
        drive_file,
        file_tree_drive::{self, errors::FileIdentifier, FileTreeDrive},
        hub_helper::get_hub,
        md5_writer::Md5Writer,
        parse_md5_digest, FileTreeLike, FolderLike,
    },
    files,
    hub::Hub,
};

pub struct Config {
    pub file_id: String,
    pub existing_file_action: ExistingFileAction,
    pub follow_shortcuts: bool,
    pub download_directories: bool,
    pub destination: Destination,
}

impl Config {
    fn canonical_destination_root(&self) -> Result<PathBuf, errors::Download> {
        use errors::Download as E;

        match &self.destination {
            Destination::CurrentDir => {
                let current_path = PathBuf::from(".");
                let canonical_current_path = current_path
                    .canonicalize()
                    .map_err(|err| E::CanonicalizeDestinationPath(current_path.clone(), err))?;
                Ok(canonical_current_path)
            }

            Destination::Path(path) => {
                if !path.exists() {
                    Err(E::DestinationPathDoesNotExist(path.clone()))
                } else if !path.is_dir() {
                    Err(E::DestinationPathNotADirectory(path.clone()))
                } else {
                    path.canonicalize()
                        .map_err(|err| E::CanonicalizeDestinationPath(path.clone(), err))
                }
            }

            Destination::Stdout => Err(E::StdoutNotValidDestination),
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
pub async fn download(config: Config) -> Result<(), errors::Download> {
    use errors::Download as E;

    let hub = get_hub().await.map_err(E::Hub)?;

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| E::GetFile(Box::new(err)))?;

    err_if_file_exists(&file, &config)?;

    if drive_file::is_shortcut(&file) {
        if !config.follow_shortcuts {
            return Err(E::IsShortcut(FileIdentifier::from(file)));
        }

        let google_drive3::api::File {
            shortcut_details,
            name,
            id,
            ..
        } = file;
        let target_file_id = shortcut_details.and_then(|details| details.target_id);

        let file_id = target_file_id
            .ok_or_else(|| E::MissingShortcutTarget(FileIdentifier::new(name, id)))?;

        download(Config { file_id, ..config }).await?;
    } else if drive_file::is_directory(&file) {
        if !config.download_directories {
            return Err(E::IsDirectory(FileIdentifier::from(file)));
        }

        download_directory(&hub, file, &config).await?;
    } else {
        download_regular(&hub, &file, &config).await?;
    }

    Ok(())
}

async fn download_regular(
    hub: &Hub,
    file: &google_drive3::api::File,
    config: &Config,
) -> Result<(), errors::Download> {
    use errors::Download as E;

    let body = download_file(hub, &config.file_id)
        .await
        .map_err(|err| E::DownloadFile(Box::new(err)))?;

    if config.destination == Destination::Stdout {
        save_body_to_stdout(body).await?;
    } else {
        let file_name = file
            .name
            .as_deref()
            .ok_or_else(|| E::MissingFileName(FileIdentifier::from(file)))?;
        let mut abs_file_path = config.canonical_destination_root()?;
        abs_file_path.push(file_name);

        println!("Downloading {file_name}");
        let md5_checksum = file.md5_checksum.as_deref().and_then(parse_md5_digest);
        if let Err(source) = save_body_to_file(body, &abs_file_path, md5_checksum.as_ref()).await {
            return Err(E::SaveBodyToFile {
                path: abs_file_path,
                source,
            });
        }
        println!("Successfully downloaded {file_name}");
    }

    Ok(())
}

async fn download_directory(
    hub: &Hub,
    file: google_drive3::api::File,
    config: &Config,
) -> Result<(), errors::Download> {
    use errors::Download as E;

    let tree = FileTreeDrive::from_file(hub, file)
        .await
        .map_err(E::CreateFileTree)?;

    let tree_info = tree.info();

    println!(
        "Found {} files in {} directories with a total size of {}",
        tree_info.file_count,
        tree_info.folder_count,
        ByteSize::b(tree_info.total_file_size).display().si(),
    );

    let root_path = config.canonical_destination_root()?;

    for folder in &tree.folders() {
        let folder_path = folder.info.relative_path();
        let abs_folder_path = root_path.join(&folder_path);

        println!("Creating directory {}", folder_path.display());
        fs::create_dir_all(&abs_folder_path)
            .await
            .map_err(|err| E::CreateDirectory(abs_folder_path, err))?;

        for file in folder.files() {
            let file_path = file.relative_path();
            let abs_file_path = root_path.join(&file_path);

            if local_file_is_identical(&abs_file_path, &file).await {
                continue;
            }

            let body = download_file(hub, &file.drive_id)
                .await
                .map_err(|err| E::DownloadFile(Box::new(err)))?;

            println!("Downloading file '{}'", file_path.display());
            if let Err(source) = save_body_to_file(body, &abs_file_path, file.md5.as_ref()).await {
                return Err(E::SaveBodyToFile {
                    path: abs_file_path,
                    source,
                });
            }
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

async fn download_file(hub: &Hub, file_id: &str) -> Result<hyper::Body, google_drive3::Error> {
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

// TODO: move to common
pub async fn save_body_to_file(
    mut body: hyper::Body,
    file_path: &Path,
    expected_md5: Option<&Digest>,
) -> Result<(), errors::SaveBodyToFile> {
    use errors::SaveBodyToFile as E;

    // Create temporary file
    let tmp_file_path = file_path.with_extension("incomplete");
    let file = File::create(&tmp_file_path).await.map_err(E::CreateFile)?;

    // Wrap file in writer that calculates md5
    let mut writer = Md5Writer::new(file);

    // Read chunks from stream and write to file
    while let Some(chunk_result) = body.next().await {
        let chunk = chunk_result.map_err(E::ReadChunk)?;
        writer.write_all(&chunk).await.map_err(E::WriteChunk)?;
    }

    // Check md5
    let md5_digest = writer.md5();
    if let Some(expected_md5) = expected_md5 {
        if *expected_md5 != md5_digest {
            return Err(E::Md5Mismatch {
                expected: *expected_md5,
                actual: md5_digest,
            });
        }
    }

    // Rename temporary file to final file
    fs::rename(&tmp_file_path, file_path)
        .await
        .map_err(E::RenameFile)
}

// TODO: move to common
pub async fn save_body_to_stdout(mut body: hyper::Body) -> Result<(), errors::SaveBodyToStdout> {
    let mut stdout = io::stdout();

    // Read chunks from stream and write to stdout
    while let Some(chunk_result) = body.next().await {
        let chunk = chunk_result.map_err(errors::SaveBodyToStdout::ReadChunk)?;
        stdout
            .write_all(&chunk)
            .await
            .map_err(errors::SaveBodyToStdout::WriteChunk)?;
    }

    Ok(())
}

fn err_if_file_exists(
    file: &google_drive3::api::File,
    config: &Config,
) -> Result<(), errors::Download> {
    let file_name = file
        .name
        .as_deref()
        .ok_or_else(|| errors::Download::MissingFileName(FileIdentifier::from(file)))?;

    let file_path = match &config.destination {
        Destination::CurrentDir => Some(Path::new(".").join(file_name)),
        Destination::Path(path) => Some(path.join(file_name)),
        Destination::Stdout => None,
    };

    match file_path {
        Some(path) => {
            if path.exists() && config.existing_file_action == ExistingFileAction::Abort {
                Err(errors::Download::FileExists(FileIdentifier::from(file)))
            } else {
                Ok(())
            }
        }

        None => Ok(()),
    }
}

async fn local_file_is_identical(path: &Path, file: &file_tree_drive::File) -> bool {
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

async fn compute_md5_from_path(path: &Path) -> Result<Digest, io::Error> {
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
