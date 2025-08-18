use std::{
    error::Error,
    fmt::{self, Display},
    num::TryFromIntError,
};

use crate::files;

#[derive(Debug)]
pub struct FileTreeDrive(pub Folder);

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

#[derive(Debug)]
pub enum Folder {
    MissingFileName,
    NotDirectory,
    MissingFileId,
    ListFiles(files::list::Error),
    Nested {
        identifier: FileIdentifier,
        source: Box<Folder>,
    },
    File {
        identifier: FileIdentifier,
        source: File,
    },
}

impl Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Folder::MissingFileName => f.write_str("file name is missing"),
            Folder::NotDirectory => f.write_str("file is not a directory"),
            Folder::MissingFileId => f.write_str("file id is missing"),
            Folder::ListFiles(_) => f.write_str("unable to list directory files"),
            Folder::Nested {
                identifier,
                source: _,
            } => {
                write!(
                    f,
                    "unable to process nested directory{}",
                    identifier.display()
                )
            }
            Folder::File {
                identifier,
                source: _,
            } => {
                write!(f, "unable to process file{}", identifier.display())
            }
        }
    }
}

impl Error for Folder {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Folder::MissingFileName | Folder::NotDirectory | Folder::MissingFileId => None,
            Folder::ListFiles(error) => Some(error),
            Folder::Nested { source, .. } => Some(source),
            Folder::File { source, .. } => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum File {
    MissingFileName,
    MissingFileSize,
    InvalidFileSize(TryFromIntError),
    MissingFileId,
}

impl Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            File::MissingFileName => "file name is missing",
            File::MissingFileSize => "file size is missing",
            File::InvalidFileSize(_) => "file size is invalid",
            File::MissingFileId => "file id is missing",
        };

        f.write_str(s)
    }
}

impl Error for File {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            File::MissingFileName | File::MissingFileSize | File::MissingFileId => None,
            File::InvalidFileSize(source) => Some(source),
        }
    }
}
