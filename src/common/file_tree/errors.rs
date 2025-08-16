use std::{
    error::Error,
    fmt::{self, Display},
    io,
    path::PathBuf,
};

use crate::common::id_gen;

#[derive(Debug)]
pub enum FileTree {
    Canonicalize(io::Error),
    Folder(Folder),
}

impl Display for FileTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FileTree::Canonicalize(_) => "unable to canonicalize path",
            FileTree::Folder(_) => "unable to create folder tree from canonicalized path",
        };

        f.write_str(s)
    }
}

impl Error for FileTree {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileTree::Canonicalize(source) => Some(source),
            FileTree::Folder(source) => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum Folder {
    InvalidPath,
    GenerateId(id_gen::Error),
    ReadDir(io::Error),
    ReadDirEntry(io::Error),
    Nested { path: PathBuf, source: Box<Folder> },
    IsSymlink(PathBuf),
    File { path: PathBuf, source: File },
    UnknownFileType(PathBuf),
}

impl Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Folder::InvalidPath => f.write_str("directory name is invalid"),
            Folder::GenerateId(_) => f.write_str("unable to generate google drive id"),
            Folder::ReadDir(_) => f.write_str("unable to read directory content"),
            Folder::ReadDirEntry(_) => f.write_str("unable to read entry from directory"),
            Folder::Nested { path, source: _ } => {
                write!(f, "cannot evaluate child directory '{}", path.display())
            }
            Folder::IsSymlink(path) => write!(
                f,
                "file '{}' is a symlink, symlinks are not supported",
                path.display()
            ),
            Folder::File { path, source: _ } => {
                write!(f, "unable to evaluate file '{}'", path.display())
            }
            Folder::UnknownFileType(path) => write!(
                f,
                "file '{}' is not regular, a directory or a symlink",
                path.display()
            ),
        }
    }
}

impl Error for Folder {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Folder::InvalidPath | Folder::IsSymlink(_) | Folder::UnknownFileType(_) => None,
            Folder::GenerateId(source) => Some(source),
            Folder::ReadDir(source) | Folder::ReadDirEntry(source) => Some(source),
            Folder::Nested { source, .. } => Some(source),
            Folder::File { source, .. } => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum File {
    InvalidPath,
    OpenFile(io::Error),
    GenerateId(id_gen::Error),
}

impl Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            File::InvalidPath => "file path is invalid",
            File::OpenFile(_) => "unable to open file",
            File::GenerateId(_) => "unable to generate google drive id",
        };

        f.write_str(s)
    }
}

impl Error for File {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            File::InvalidPath => None,
            File::OpenFile(source) => Some(source),
            File::GenerateId(source) => Some(source),
        }
    }
}
