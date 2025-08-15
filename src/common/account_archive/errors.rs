use std::{
    error::Error,
    fmt::{self, Display},
    io,
    path::PathBuf,
};

#[derive(Debug)]
pub enum Create {
    SrcPathDoesNotExist,
    SrcPathNotDirectory,
    CreateArchive(io::Error),
    AppendDir {
        dir_path: PathBuf,
        source: io::Error,
    },
    FinishArchive(io::Error),
}

impl Display for Create {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Create::SrcPathDoesNotExist => f.write_str("source path does not exist"),
            Create::SrcPathNotDirectory => f.write_str("source path is not a directory"),
            Create::CreateArchive(_) => f.write_str("unable to create the archive file"),
            Create::AppendDir {
                dir_path,
                source: _,
            } => write!(
                f,
                "unable to append directory '{}' to archive",
                dir_path.display()
            ),
            Create::FinishArchive(_) => f.write_str("unable to finish creating the archive"),
        }
    }
}

impl Error for Create {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Create::SrcPathDoesNotExist | Create::SrcPathNotDirectory => None,
            Create::CreateArchive(source)
            | Create::FinishArchive(source)
            | Create::AppendDir { source, .. } => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum Unpack {
    ArchivePathDoesNotExist,
    DstDoesNotExist,
    Open(io::Error),
    Unpack(io::Error),
}

impl Display for Unpack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Unpack::ArchivePathDoesNotExist => "archive path does not exist",
            Unpack::DstDoesNotExist => "destination path does not exist",
            Unpack::Open(_) => "unable to open the archive file",
            Unpack::Unpack(_) => "unable to unpack the archive",
        };

        f.write_str(s)
    }
}

impl Error for Unpack {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Unpack::ArchivePathDoesNotExist | Unpack::DstDoesNotExist => None,
            Unpack::Open(source) | Unpack::Unpack(source) => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum GetAccountName {
    Open(io::Error),
    ReadEntries(io::Error),
    NoDirectories,
    MultipleDirectories,
}

impl Display for GetAccountName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            GetAccountName::Open(_) => "unable to open archive file",
            GetAccountName::ReadEntries(_) => "unable to read entries from archive",
            GetAccountName::NoDirectories => "archive contains no directories",
            GetAccountName::MultipleDirectories => "archive contains more than one directory",
        };

        f.write_str(s)
    }
}

impl Error for GetAccountName {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GetAccountName::Open(source) | GetAccountName::ReadEntries(source) => Some(source),
            GetAccountName::NoDirectories | GetAccountName::MultipleDirectories => None,
        }
    }
}
