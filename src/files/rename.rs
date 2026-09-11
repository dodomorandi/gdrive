use std::{
    error,
    fmt::{Display, Formatter},
};

use crate::{
    common::{
        delegate::UploadDelegateConfig,
        hub_helper::{get_hub, GetHubError},
    },
    files::{self, update::PatchFile},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub file_id: String,
    pub name: String,
}

pub async fn rename(config: Config) -> Result<(), Box<Error>> {
    let hub = get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let old_file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(Error::GetFile)?;

    println!(
        "Renaming {} to {}",
        old_file.name.unwrap_or_default(),
        config.name
    );

    let patch_file = PatchFile::new(config.file_id).with_name(config.name);

    files::update::update_metadata(&hub, &delegate_config, patch_file)
        .await
        .map_err(Error::Rename)?;

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    GetFile(google_drive3::Error),
    Rename(google_drive3::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source) | Error::Rename(source) => Some(source),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::GetFile(_) => f.write_str("unable to get file"),
            Error::Rename(_) => f.write_str("unable to rename file"),
        }
    }
}
