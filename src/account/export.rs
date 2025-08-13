use error_trace::ErrorTrace;

use crate::app_config;
use crate::app_config::set_file_permissions;
use crate::app_config::AppConfig;
use crate::common::account_archive;
use std::borrow::Cow;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Write;
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
        return Err(Error::AccountNotFound);
    }

    let app_cfg = AppConfig::init_account(account_name).map_err(Error::InitAccount)?;
    let account_path = app_cfg.account_base_path();

    let archive_name = format!("gdrive_export-{}.tar", normalize_name(account_name));
    let archive_path = Path::new(&archive_name);
    account_archive::create(&account_path, archive_path).map_err(Error::CreateArchive)?;

    if let Err(err) = set_file_permissions(archive_path) {
        eprintln!(
            "Warning: Failed to set permissions on archive: {}",
            err.trace()
        );
    }

    println!("Exported account '{account_name}' to {archive_name}");

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ListAccounts(app_config::Error),
    InitAccount(app_config::Error),
    AccountNotFound,
    CreateArchive(account_archive::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ListAccounts(error) | Error::InitAccount(error) => Some(error),
            Error::AccountNotFound => None,
            Error::CreateArchive(error) => Some(error),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ListAccounts(_) => f.write_str("unable to list accounts from config"),
            Error::InitAccount(_) => f.write_str("unable to initialize account in config"),
            Error::AccountNotFound => f.write_str("account not found"),
            Error::CreateArchive(_) => f.write_str("unable to create account archive"),
        }
    }
}

fn normalize_name(account_name: &str) -> Cow<'_, str> {
    match account_name
        .char_indices()
        .find_map(|(index, c)| c.is_alphanumeric().not().then_some(index))
    {
        None => Cow::Borrowed(account_name),
        Some(index) => {
            let mut normalized = String::with_capacity(account_name.len());
            let (init, rest) = account_name.split_at(index);
            write!(normalized, "{init}_").unwrap();
            normalized.extend(
                rest.chars()
                    .skip(1)
                    .map(|c| if c.is_alphanumeric() { c } else { '_' }),
            );
            Cow::Owned(normalized)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn normalize_name() {
        assert_eq!(super::normalize_name("hello123"), "hello123");
        assert_eq!(
            super::normalize_name("smile 😀! It's fine"),
            "smile____It_s_fine"
        );
    }
}
