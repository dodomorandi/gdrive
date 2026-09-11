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
    files::{self, FileResult},
    hub::Hub,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub file_id: String,
    pub to_folder_id: String,
}

pub async fn mv(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let old_file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    let old_parent_id = get_old_parent_id(&old_file)?;

    let old_parent = files::info::get_file(&hub, old_parent_id)
        .await
        .map_err(|err| Error::GetOldParent(old_parent_id.to_owned(), Box::new(err)))?;

    let new_parent = files::info::get_file(&hub, &config.to_folder_id)
        .await
        .map_err(|err| Error::GetNewParent(Box::new(err)))?;

    if drive_file::is_directory(&new_parent).not() {
        return Err(Error::NotADirectory);
    }

    println!(
        "Moving '{}' from '{}' to '{}'",
        old_file.name.as_deref().unwrap_or_default(),
        old_parent.name.unwrap_or_default(),
        new_parent.name.unwrap_or_default()
    );

    let change_parent_config = ChangeParentConfig {
        file_id: &config.file_id,
        old_parent_id,
        new_parent_id: &config.to_folder_id,
    };

    change_parent(&hub, &delegate_config, change_parent_config)
        .await
        .map_err(|err| Error::Move(Box::new(err)))?;

    Ok(())
}

pub struct ChangeParentConfig<'a> {
    pub file_id: &'a str,
    pub old_parent_id: &'a str,
    pub new_parent_id: &'a str,
}

#[expect(
    clippy::result_large_err,
    reason = "Ok variant is bigger, see test next to FileResult"
)]
pub async fn change_parent(
    hub: &Hub,
    delegate_config: &UploadDelegateConfig,
    config: ChangeParentConfig<'_>,
) -> FileResult {
    let mut delegate = UploadDelegate::new(delegate_config);

    let empty_file = google_drive3::api::File::default();

    let (_, file) = hub
        .files()
        .update(empty_file, config.file_id)
        .remove_parents(config.old_parent_id)
        .add_parents(config.new_parent_id)
        .param("fields", "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,description,webContentLink,webViewLink")
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true)
        .doit_without_upload().await?;

    Ok(file)
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    GetFile(Box<google_drive3::Error>),
    GetOldParent(String, Box<google_drive3::Error>),
    GetNewParent(Box<google_drive3::Error>),
    NoParents,
    MultipleParents,
    NotADirectory,
    Move(Box<google_drive3::Error>),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source)
            | Error::GetOldParent(_, source)
            | Error::GetNewParent(source)
            | Error::Move(source) => Some(source),
            Error::NoParents | Error::MultipleParents | Error::NotADirectory => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::GetFile(_) => f.write_str("unable to get file"),
            Error::GetNewParent(_) => f.write_str("unable to get new parent"),
            Error::GetOldParent(id, _) => {
                write!(f, "unable to get old parent '{id}'")
            }
            Error::NoParents => f.write_str("file has no parents"),
            Error::MultipleParents => f.write_str("can't move file with multiple parents"),
            Error::NotADirectory => f.write_str("new parent is not a directory"),
            Error::Move(_) => f.write_str("unable to move file"),
        }
    }
}

fn get_old_parent_id(file: &google_drive3::api::File) -> Result<&'_ str, Error> {
    match &file.parents {
        None => Err(Error::NoParents),

        Some(parents) => match &parents[..] {
            [] => Err(Error::NoParents),
            [parent_id] => Ok(parent_id.as_str()),
            _ => Err(Error::MultipleParents),
        },
    }
}
