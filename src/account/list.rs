use std::{
    error,
    fmt::{Display, Formatter},
};

use crate::app_config;

pub fn list() -> Result<(), Error> {
    let accounts = app_config::list_accounts().map_err(Error::ListAccounts)?;
    if accounts.is_empty() {
        return Err(Error::NoAccounts);
    }

    for account in accounts {
        println!("{account}");
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    ListAccounts(app_config::errors::ListAccounts),
    NoAccounts,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ListAccounts(source) => Some(source),
            Error::NoAccounts => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::ListAccounts(_) => "unable to list accounts",
            Error::NoAccounts => "no accounts found; use `gdrive account add` to add an account",
        };
        f.write_str(s)
    }
}
