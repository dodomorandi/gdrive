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

impl Display for CreateAccountDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unable to create account directory")
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
