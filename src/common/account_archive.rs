use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub fn create(src_path: &Path, archive_path: &Path) -> Result<(), Error> {
    err_if_not_exists(src_path)?;
    err_if_not_dir(src_path)?;
    err_if_exists(archive_path)?;

    let archive_file = File::create(archive_path).map_err(Error::CreateFile)?;
    let mut builder = tar::Builder::new(archive_file);

    let src_dir_name = src_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    builder
        .append_dir_all(&src_dir_name, src_path)
        .map_err(|err| Error::AppendDir(src_path.to_path_buf(), err))?;

    builder
        .finish()
        .map_err(|err| Error::FinishArchive(archive_path.to_path_buf(), err))?;

    Ok(())
}

pub fn unpack(archive_path: &Path, dst_path: &Path) -> Result<(), Error> {
    err_if_not_exists(archive_path)?;
    err_if_not_exists(dst_path)?;

    let archive_file = File::open(archive_path).map_err(Error::OpenFile)?;
    let mut archive = tar::Archive::new(archive_file);
    archive.unpack(dst_path).map_err(Error::Unpack)
}

pub fn get_account_name(archive_path: &Path) -> Result<String, Error> {
    let archive_file = File::open(archive_path).map_err(Error::OpenFile)?;
    let mut archive = tar::Archive::new(archive_file);
    let entries = archive.entries().map_err(Error::ReadEntries)?;

    let dir_names: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path().ok()?;

            if entry.header().entry_type() == tar::EntryType::Directory {
                let file_name = path.file_name()?;
                Some(file_name.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    match &dir_names[..] {
        [name] => Ok(name.to_string()),
        [] => Err(Error::NoDirectories),
        _ => Err(Error::MultipleDirectories),
    }
}

#[derive(Debug)]
pub enum Error {
    CreateFile(io::Error),
    PathDoesNotExist(PathBuf),
    PathNotDir(PathBuf),
    PathAlreadyExists(PathBuf),
    AppendDir(PathBuf, io::Error),
    FinishArchive(PathBuf, io::Error),
    OpenFile(io::Error),
    ReadEntries(io::Error),
    NoDirectories,
    MultipleDirectories,
    Unpack(io::Error),
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CreateFile(err) => {
                write!(f, "Failed to create file: {err}")
            }

            Error::PathDoesNotExist(path) => {
                write!(f, "'{}' does not exist", path.display())
            }

            Error::PathNotDir(path) => {
                write!(f, "'{}' is not a directory", path.display())
            }

            Error::PathAlreadyExists(path) => {
                write!(f, "'{}' already exists", path.display())
            }

            Error::AppendDir(path, err) => {
                write!(f, "Failed to add {} to archive: {}", path.display(), err)
            }

            Error::FinishArchive(path, err) => {
                write!(f, "Failed to create archive '{}': {}", path.display(), err)
            }

            Error::OpenFile(err) => {
                write!(f, "Failed to open archive: {err}")
            }

            Error::ReadEntries(err) => {
                write!(f, "Failed to read archive entries: {err}")
            }

            Error::NoDirectories => {
                write!(f, "Archive contains no directories")
            }

            Error::MultipleDirectories => {
                write!(f, "Archive contains multiple directories")
            }

            Error::Unpack(err) => {
                write!(f, "Failed to unpack archive: {err}")
            }
        }
    }
}

fn err_if_not_exists(path: &Path) -> Result<(), Error> {
    if path.exists() {
        Ok(())
    } else {
        Err(Error::PathDoesNotExist(path.to_owned()))
    }
}

fn err_if_not_dir(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(Error::PathNotDir(path.to_owned()))
    }
}

fn err_if_exists(path: &Path) -> Result<(), Error> {
    if path.exists() {
        Err(Error::PathAlreadyExists(path.to_owned()))
    } else {
        Ok(())
    }
}
