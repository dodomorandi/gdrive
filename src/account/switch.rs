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

pub fn switch(config: &Config) -> Result<(), Error> {
    let accounts = app_config::list_accounts().map_err(Error::ListAccounts)?;

    if accounts.contains(&config.account_name).not() {
        return Err(Error::AccountNotFound);
    }

    let app_cfg = AppConfig::init_account(&config.account_name).map_err(Error::InitAccount)?;
    app_config::switch_account(&app_cfg).map_err(Error::SwitchAccount)?;
    println!("Switched to account '{}'", &config.account_name);

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ListAccounts(app_config::Error),
    AccountNotFound,
    InitAccount(app_config::Error),
    SwitchAccount(app_config::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ListAccounts(source)
            | Error::InitAccount(source)
            | Error::SwitchAccount(source) => Some(source),
            Error::AccountNotFound => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::ListAccounts(_) => "unable to list accounts",
            Error::AccountNotFound => "account not found",
            Error::InitAccount(_) => "unable to initialize the account",
            Error::SwitchAccount(_) => "unable to switch to the account",
        };

        f.write_str(s)
    }
}
