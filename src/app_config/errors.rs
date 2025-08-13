use std::{
    error::Error,
    fmt::{self, Display},
    io,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeDirNotFound;

impl Display for HomeDirNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("home directory not found")
    }
}

impl Error for HomeDirNotFound {}

impl From<HomeDirNotFound> for super::Error {
    fn from(HomeDirNotFound: HomeDirNotFound) -> Self {
        super::Error::HomeDirNotFound
    }
}

#[derive(Debug)]
pub struct CreateAccountDir(pub io::Error);

impl CreateAccountDir {
    const DISPLAY: &str = "unable to create account directory";
}

impl Display for CreateAccountDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::DISPLAY)
    }
}

impl Error for CreateAccountDir {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<CreateAccountDir> for super::Error {
    fn from(value: CreateAccountDir) -> Self {
        super::Error::CreateConfigDir(value.0)
    }
}

#[derive(Debug)]
pub enum InitAccount {
    DefaultBasePath(HomeDirNotFound),
    CreateAccountDir(io::Error),
}

impl Display for InitAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InitAccount::DefaultBasePath(_) => "unable to get default base path",
            InitAccount::CreateAccountDir(_) => CreateAccountDir::DISPLAY,
        };

        f.write_str(s)
    }
}

impl Error for InitAccount {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InitAccount::DefaultBasePath(source) => Some(source),
            InitAccount::CreateAccountDir(source) => Some(source),
        }
    }
}

impl From<HomeDirNotFound> for InitAccount {
    fn from(value: HomeDirNotFound) -> Self {
        InitAccount::DefaultBasePath(value)
    }
}

impl From<CreateAccountDir> for InitAccount {
    fn from(value: CreateAccountDir) -> Self {
        InitAccount::CreateAccountDir(value.0)
    }
}

impl From<InitAccount> for super::Error {
    fn from(value: InitAccount) -> Self {
        match value {
            InitAccount::DefaultBasePath(HomeDirNotFound) => super::Error::HomeDirNotFound,
            InitAccount::CreateAccountDir(source) => super::Error::CreateConfigDir(source),
        }
    }
}
