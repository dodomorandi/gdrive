use mktemp::Temp;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io;
use std::path::PathBuf;

pub fn stdin_to_file() -> Result<Temp, io::Error> {
    let tmp_file = Temp::new_file()?;
    let path = tmp_file.as_ref().to_path_buf();
    let mut file = File::create(&path)?;
    io::copy(&mut io::stdin(), &mut file)?;
    Ok(tmp_file)
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
    StdinToFile(io::Error),
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
            OpenFileError::Open { source, .. } | OpenFileError::StdinToFile(source) => Some(source),
        }
    }
}
