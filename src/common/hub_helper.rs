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
    let app_cfg = AppConfig::load_current_account().map_err(Error::AppConfig)?;
    let secret = app_cfg.load_secret().map_err(Error::AppConfig)?;
    let auth = Auth::new(&secret, &app_cfg.tokens_path())
        .await
        .map_err(Error::Auth)?;

    let hub = Hub::new(auth).await.map_err(Error::Hub)?;

    Ok(hub)
}

#[derive(Debug)]
pub enum Error {
    AppConfig(app_config::Error),
    Auth(io::Error),
    Hub(io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(error) => Some(error),
            // TODO: fix display and put sources here
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::AppConfig(err) => write!(f, "{err}"),
            Error::Auth(err) => write!(f, "Auth error: {err}"),
            Error::Hub(_) => f.write_str("unable to create Google Drive hub"),
        }
    }
}
