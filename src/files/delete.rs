use std::{
    error,
    fmt::{Display, Formatter},
};

use crate::{
    common::{
        drive_file,
        file_tree_drive::errors::FileIdentifier,
        hub_helper::{get_hub, GetHubError},
    },
    files,
};

pub struct Config {
    pub file_id: String,
    pub delete_directories: bool,
}

pub async fn delete(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    if drive_file::is_directory(&file) && !config.delete_directories {
        return Err(Error::IsDirectory(FileIdentifier::from(file)));
    }

    hub.files()
        .delete(&config.file_id)
        .supports_all_drives(true)
        .add_scope(google_drive3::api::Scope::Full)
        .doit()
        .await
        .map_err(|err| Error::DeleteFile(Box::new(err)))?;

    println!("Deleted '{}'", file.name.unwrap_or_default());

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    GetFile(Box<google_drive3::Error>),
    DeleteFile(Box<google_drive3::Error>),
    IsDirectory(FileIdentifier),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::GetFile(_) => f.write_str("unable to get file to delete"),
            Error::DeleteFile(_) => f.write_str("unable to delete file"),
            Error::IsDirectory(identifier) => write!(
                f,
                "file{} is a directory, use --recursive to delete directories",
                identifier.display(),
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source) | Error::DeleteFile(source) => Some(source),
            Error::IsDirectory(_) => None,
        }
    }
}
