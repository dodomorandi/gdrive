use crate::app_config;
use crate::app_config::AppConfig;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;

pub fn current() -> Result<(), Error> {
    let accounts = app_config::list_accounts().map_err(Error::List)?;
    if accounts.is_empty() {
        return Err(Error::NoAccounts);
    }

    let app_cfg = AppConfig::load_current_account().map_err(Error::LoadCurrent)?;
    println!("{}", app_cfg.account.name);

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    List(app_config::errors::ListAccounts),
    LoadCurrent(app_config::errors::LoadCurrentAccount),
    NoAccounts,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::List(error) => Some(error),
            Error::LoadCurrent(error) => Some(error),
            Error::NoAccounts => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::List(_) => "unable to list available accounts",
            Error::LoadCurrent(_) => "unable to load the current accont",
            Error::NoAccounts => "no accounts found; use `gdrive account add` to add an account",
        };

        f.write_str(s)
    }
}
