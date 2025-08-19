use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

use crate::{
    app_config::{self, AppConfig},
    hub::{Auth, Hub},
};

pub async fn get_hub() -> Result<Hub, GetHubError> {
    let app_cfg = AppConfig::load_current_account().map_err(GetHubError::LoadCurrentAccount)?;
    let secret = app_cfg.load_secret().map_err(GetHubError::LoadSecret)?;
    let auth = Auth::new(&secret, app_cfg.tokens_path())
        .await
        .map_err(GetHubError::Auth)?;

    let hub = Hub::new(auth).map_err(GetHubError::Hub)?;

    Ok(hub)
}

#[derive(Debug)]
pub enum GetHubError {
    LoadCurrentAccount(app_config::errors::LoadCurrentAccount),
    LoadSecret(app_config::errors::LoadSecret),
    Auth(io::Error),
    Hub(io::Error),
}

impl Error for GetHubError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GetHubError::LoadCurrentAccount(source) => Some(source),
            GetHubError::LoadSecret(source) => Some(source),
            GetHubError::Hub(source) | GetHubError::Auth(source) => Some(source),
        }
    }
}

impl Display for GetHubError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            GetHubError::LoadCurrentAccount(_) => "unable to load current account",
            GetHubError::LoadSecret(_) => "unable to load secret",
            GetHubError::Auth(_) => "unable to authenticate",
            GetHubError::Hub(_) => "unable to create Google Drive hub",
        };

        f.write_str(s)
    }
}
