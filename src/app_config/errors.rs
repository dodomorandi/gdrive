use std::{
    error::Error,
    fmt::{self, Display},
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
