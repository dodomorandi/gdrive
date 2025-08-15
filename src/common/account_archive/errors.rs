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
