use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug)]
pub struct FileTreeDrive(pub super::Error);

impl Display for FileTreeDrive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unable to create a folder from file")
    }
}

impl Error for FileTreeDrive {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileIdentifier {
    Name(String),
    Id(String),
    None,
}

impl FileIdentifier {
    #[must_use]
    pub fn new(name: Option<String>, id: Option<String>) -> Self {
        if let Some(name) = name {
            Self::Name(name)
        } else if let Some(id) = id {
            Self::Id(id)
        } else {
            Self::None
        }
    }

    #[must_use]
    pub fn display(&self) -> FileIdentifierDisplay<'_> {
        FileIdentifierDisplay(self)
    }
}

impl From<google_drive3::api::File> for FileIdentifier {
    fn from(value: google_drive3::api::File) -> Self {
        Self::new(value.name, value.id)
    }
}

impl From<&google_drive3::api::File> for FileIdentifier {
    fn from(value: &google_drive3::api::File) -> Self {
        if let Some(name) = &value.name {
            Self::Name(name.clone())
        } else if let Some(id) = &value.id {
            Self::Id(id.clone())
        } else {
            Self::None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentifierDisplay<'a>(&'a FileIdentifier);

impl Display for FileIdentifierDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            FileIdentifier::Name(name) => write!(f, " with name '{name}'"),
            FileIdentifier::Id(id) => write!(f, " with id '{id}'"),
            FileIdentifier::None => Ok(()),
        }
    }
}
