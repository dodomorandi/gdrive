use crate::app_config;
use crate::app_config::AppConfig;
use crate::hub::Auth;
use crate::hub::Hub;
use std::error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io;

pub async fn get_hub() -> Result<Hub, Error> {
    let app_cfg = AppConfig::load_current_account().map_err(Error::LoadCurrentAccount)?;
    let secret = app_cfg.load_secret().map_err(Error::LoadSecret)?;
    let auth = Auth::new(&secret, app_cfg.tokens_path())
        .await
        .map_err(Error::Auth)?;

    let hub = Hub::new(auth).map_err(Error::Hub)?;

    Ok(hub)
}

#[derive(Debug)]
pub enum Error {
    LoadCurrentAccount(app_config::errors::LoadCurrentAccount),
    LoadSecret(app_config::errors::LoadSecret),
    Auth(io::Error),
    Hub(io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::LoadCurrentAccount(source) => Some(source),
            Error::LoadSecret(source) => Some(source),
            Error::Hub(error) => Some(error),
            // FIXME
            Error::Auth(_) => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::LoadCurrentAccount(_) => f.write_str("unable to load current account"),
            Error::LoadSecret(_) => f.write_str("unable to load secret"),
            Error::Auth(err) => write!(f, "Auth error: {err}"),
            Error::Hub(_) => f.write_str("unable to create Google Drive hub"),
        }
    }
}
