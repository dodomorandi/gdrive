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
    let accounts = app_config::list_accounts().map_err(Error::AppConfig)?;

    if accounts.contains(&config.account_name).not() {
        return Err(Error::AccountNotFound(config.account_name.clone()));
    }

    let app_cfg = AppConfig::init_account(&config.account_name).map_err(Error::AppConfig)?;
    app_config::switch_account(&app_cfg).map_err(Error::AppConfig)?;
    println!("Switched to account '{}'", &config.account_name);

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    AppConfig(app_config::Error),
    AccountNotFound(String),
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AppConfig(e) => write!(f, "{e}"),
            Error::AccountNotFound(name) => write!(f, "Account '{name}' not found"),
        }
    }
}
