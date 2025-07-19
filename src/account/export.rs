use crate::app_config;
use crate::app_config::set_file_permissions;
use crate::app_config::AppConfig;
use crate::common::account_archive;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Not;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub account_name: String,
}

pub fn export(config: &Config) -> Result<(), Error> {
    let Config { account_name } = config;
    let accounts = app_config::list_accounts().map_err(Error::ListAccounts)?;
    if accounts.contains(account_name).not() {
        return Err(Error::AccountNotFound(account_name.clone()));
    }

    let app_cfg = AppConfig::init_account(account_name).map_err(Error::InitAccount)?;
    let account_path = app_cfg.account_base_path();

    let archive_name = format!("gdrive_export-{}.tar", normalize_name(account_name));
    let archive_path = Path::new(&archive_name);
    account_archive::create(&account_path, archive_path).map_err(Error::CreateArchive)?;

    if let Err(err) = set_file_permissions(archive_path) {
        eprintln!("Warning: Failed to set permissions on archive: {err}");
    }

    println!("Exported account '{account_name}' to {archive_name}");

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ListAccounts(app_config::Error),
    InitAccount(app_config::Error),
    AccountNotFound(String),
    CreateArchive(account_archive::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ListAccounts(error) | Error::InitAccount(error) => Some(error),
            Error::AccountNotFound(_) => None,
            Error::CreateArchive(error) => Some(error),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ListAccounts(_) => f.write_str("unable to list accounts from config"),
            Error::InitAccount(_) => f.write_str("unable to initialize account in config"),
            Error::AccountNotFound(name) => write!(f, "Account '{name}' not found"),
            Error::CreateArchive(_) => f.write_str("unable to create account archive"),
        }
    }
}

fn normalize_name(account_name: &str) -> String {
    account_name
        .chars()
        .map(|c| if char::is_alphanumeric(c) { c } else { '_' })
        .collect()
}
