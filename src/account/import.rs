use crate::app_config;
use crate::app_config::AppConfig;
use crate::common::account_archive;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub archive_path: PathBuf,
}

pub fn import(config: &Config) -> Result<(), Error> {
    let account_name =
        account_archive::get_account_name(&config.archive_path).map_err(Error::ReadAccountName)?;

    let accounts = app_config::list_accounts().map_err(Error::ListAccounts)?;
    if accounts.contains(&account_name) {
        return Err(Error::AccountExists(account_name.to_string()));
    }

    let config_base_path = AppConfig::default_base_path().map_err(Error::DefaultBasePath)?;
    account_archive::unpack(&config.archive_path, &config_base_path).map_err(Error::Unpack)?;

    println!("Imported account '{account_name}'");

    if !AppConfig::has_current_account() {
        let app_cfg = AppConfig::load_account(&account_name).map_err(Error::LoadAccount)?;
        println!("Switched to account '{account_name}'");
        app_config::switch_account(&app_cfg).map_err(Error::SwitchAccount)?;
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ReadAccountName(account_archive::Error),
    ListAccounts(app_config::errors::ListAccounts),
    AccountExists(String),
    DefaultBasePath(app_config::errors::DefaultBasePath),
    Unpack(account_archive::Error),
    LoadAccount(app_config::errors::LoadAccount),
    SwitchAccount(app_config::errors::SaveAccountConfig),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::AccountExists(_) => None,
            Error::ReadAccountName(error) | Error::Unpack(error) => Some(error),
            Error::ListAccounts(error) => Some(error),
            Error::LoadAccount(error) => Some(error),
            Error::DefaultBasePath(error) => Some(error),
            Error::SwitchAccount(error) => Some(error),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReadAccountName(_) => f.write_str("unable to read the account name"),
            Error::ListAccounts(_) => f.write_str("unable to list accounts"),
            Error::AccountExists(name) => write!(f, "Account '{name}' already exists"),
            Error::DefaultBasePath(_) => f.write_str("unable to get the default base path"),
            Error::Unpack(_) => f.write_str("unable to unpack account archive"),
            Error::LoadAccount(_) => f.write_str("unable to load account"),
            Error::SwitchAccount(_) => f.write_str("unable to switch account"),
        }
    }
}
