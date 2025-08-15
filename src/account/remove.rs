use crate::app_config;
use crate::app_config::AppConfig;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Not;

#[derive(Debug, Clone)]
pub struct Config {
    pub account_name: String,
}

pub fn remove(config: &Config) -> Result<(), Error> {
    let accounts = app_config::list_accounts().map_err(Error::ListAccounts)?;
    if accounts.contains(&config.account_name).not() {
        return Err(Error::AccountNotFound);
    }

    let app_cfg = AppConfig::init_account(&config.account_name).map_err(Error::InitAccount)?;
    app_cfg.remove_account().map_err(Error::RemoveAccount)?;
    println!("Removed account '{}'", config.account_name);

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ListAccounts(app_config::errors::ListAccounts),
    AccountNotFound,
    InitAccount(app_config::errors::InitAccount),
    RemoveAccount(app_config::errors::RemoveAccount),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ListAccounts(source) => Some(source),
            Error::InitAccount(source) => Some(source),
            Error::AccountNotFound => None,
            Error::RemoveAccount(source) => Some(source),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::ListAccounts(_) => "unable to list accounts",
            Error::AccountNotFound => "account not found",
            Error::InitAccount(_) => "unable to initialize the account",
            Error::RemoveAccount(_) => "unable to remove the account",
        };

        f.write_str(s)
    }
}
