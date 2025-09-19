use std::{
    error,
    fmt::{Display, Formatter},
    ops::Not,
};

use crate::{
    common::{
        delegate::{UploadDelegate, UploadDelegateConfig},
        drive_file,
        hub_helper::{get_hub, GetHubError},
    },
    files::{self, info::DisplayConfig},
    hub::Hub,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub file_id: String,
    pub to_folder_id: String,
}

pub async fn copy(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    if drive_file::is_directory(&file) {
        return Err(Error::SourceIsADirectory);
    }

    let to_parent = files::info::get_file(&hub, &config.to_folder_id)
        .await
        .map_err(|err| Error::GetDestinationFolder(Box::new(err)))?;

    if drive_file::is_directory(&to_parent).not() {
        return Err(Error::DestinationNotADirectory);
    }

    println!(
        "Copying '{}' to '{}'",
        file.name.unwrap_or_default(),
        to_parent.name.unwrap_or_default()
    );

    let copy_config = CopyConfig {
        file_id: config.file_id,
        to_folder_id: config.to_folder_id,
    };

    let new_file = copy_file(&hub, &delegate_config, &copy_config)
        .await
        .map_err(|err| Error::Copy(Box::new(err)))?;

    files::info::print_file_info(&new_file, &DisplayConfig::default());

    Ok(())
}

pub struct CopyConfig {
    pub file_id: String,
    pub to_folder_id: String,
}

pub async fn copy_file(
    hub: &Hub,
    delegate_config: &UploadDelegateConfig,
    config: &CopyConfig,
) -> Result<google_drive3::api::File, google_drive3::Error> {
    let mut delegate = UploadDelegate::new(delegate_config);

    let file = google_drive3::api::File {
        parents: Some(vec![config.to_folder_id.clone()]),
        ..google_drive3::api::File::default()
    };

    let (_, file) = hub
        .files()
        .copy(file, &config.file_id)
        .param("fields", "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,description,webContentLink,webViewLink")
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true)
        .doit().await?;

    Ok(file)
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    GetFile(Box<google_drive3::Error>),
    GetDestinationFolder(Box<google_drive3::Error>),
    DestinationNotADirectory,
    SourceIsADirectory,
    Copy(Box<google_drive3::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let s = match self {
            Error::Hub(_) => "unable to get drive hub",
            Error::GetFile(_) => "unable to get source file",
            Error::SourceIsADirectory => "source is a directory",
            Error::GetDestinationFolder(_) => "unable to get destination folder",
            Error::DestinationNotADirectory => "destination is not a directory",
            Error::Copy(_) => "unable to perform the actual copy",
        };

        f.write_str(s)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source) | Error::GetDestinationFolder(source) | Error::Copy(source) => {
                Some(source)
            }
            Error::DestinationNotADirectory | Error::SourceIsADirectory => None,
        }
    }
}
