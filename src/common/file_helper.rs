use mktemp::Temp;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io;
use std::path::PathBuf;

pub fn stdin_to_file() -> Result<Temp, StdinToFileError> {
    let tmp_file = Temp::new_file().map_err(StdinToFileError::NewTempFile)?;
    let mut file = File::create(&tmp_file).map_err(StdinToFileError::CreateTempFile)?;
    io::copy(&mut io::stdin(), &mut file).map_err(StdinToFileError::CopyStdin)?;
    Ok(tmp_file)
}

#[derive(Debug)]
pub enum StdinToFileError {
    NewTempFile(io::Error),
    CreateTempFile(io::Error),
    CopyStdin(io::Error),
}

impl Display for StdinToFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StdinToFileError::NewTempFile(_) => "unable to create a new temporary file",
            StdinToFileError::CreateTempFile(_) => {
                "unable to open and truncate the new temporary file"
            }
            StdinToFileError::CopyStdin(_) => "unable to copy stdin to temporary file",
        };

        f.write_str(s)
    }
}

impl Error for StdinToFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StdinToFileError::NewTempFile(source)
            | StdinToFileError::CreateTempFile(source)
            | StdinToFileError::CopyStdin(source) => Some(source),
        }
    }
}

pub fn open_file(path: &Option<PathBuf>) -> Result<(File, PathBuf), OpenFileError> {
    if let Some(path) = path {
        let file = File::open(path).map_err(|source| OpenFileError::Open {
            path: path.clone(),
            source,
        })?;
        Ok((file, path.clone()))
    } else {
        let tmp_file = stdin_to_file().map_err(OpenFileError::StdinToFile)?;
        let path = tmp_file.as_ref().to_path_buf();
        match File::open(&path) {
            Ok(file) => Ok((file, path)),
            Err(source) => Err(OpenFileError::Open { path, source }),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    Open { path: PathBuf, source: io::Error },
    StdinToFile(StdinToFileError),
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenFileError::Open { path, source: _ } => {
                write!(f, "unable to open path '{}'", path.display())
            }
            OpenFileError::StdinToFile(_) => f.write_str("unable to create a file from stdin"),
        }
    }
}

impl Error for OpenFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OpenFileError::Open { source, .. } => Some(source),
            OpenFileError::StdinToFile(source) => Some(source),
        }
    }
}
