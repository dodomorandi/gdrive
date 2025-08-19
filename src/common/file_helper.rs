use std::{
    error::Error,
    fmt::{self, Display},
    fs,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use mktemp::Temp;

pub fn stdin_to_file() -> Result<Temp, StdinToFileError> {
    let tmp_file = Temp::new_file().map_err(StdinToFileError::NewTempFile)?;
    let mut file = fs::File::create(&tmp_file).map_err(StdinToFileError::CreateTempFile)?;
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

pub fn open_file(path: &Option<PathBuf>) -> Result<File<'_>, OpenFileError> {
    let (file, kind) = if let Some(path) = path {
        let file = fs::File::open(path).map_err(|source| OpenFileError::Open {
            path: path.clone(),
            source,
        })?;
        (file, FileKind::File(path))
    } else {
        let tmp_file = stdin_to_file().map_err(OpenFileError::StdinToFile)?;
        let file = match fs::File::open(&tmp_file) {
            Ok(file) => file,
            Err(source) => {
                return Err(OpenFileError::Open {
                    path: tmp_file.release(),
                    source,
                })
            }
        };

        (file, FileKind::Temp(tmp_file))
    };

    Ok(File { inner: file, kind })
}

#[derive(Debug)]
pub struct File<'a> {
    inner: fs::File,
    kind: FileKind<'a>,
}

#[derive(Debug)]
enum FileKind<'a> {
    Temp(Temp),
    File(&'a Path),
}

impl File<'_> {
    #[must_use]
    pub fn path(&self) -> &Path {
        self.kind.path()
    }

    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        match self.kind {
            FileKind::Temp(temp) => temp.release(),
            FileKind::File(path) => path.to_path_buf(),
        }
    }

    #[must_use]
    pub fn file_mut_and_path(&mut self) -> (&mut fs::File, &Path) {
        (&mut self.inner, self.kind.path())
    }
}

impl AsRef<fs::File> for File<'_> {
    fn as_ref(&self) -> &fs::File {
        &self.inner
    }
}

impl Read for File<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)
    }
}

impl Seek for File<'_> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }

    fn rewind(&mut self) -> io::Result<()> {
        self.inner.rewind()
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        self.inner.stream_position()
    }

    fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        self.inner.seek_relative(offset)
    }
}

impl FileKind<'_> {
    fn path(&self) -> &Path {
        match self {
            FileKind::Temp(temp) => temp,
            FileKind::File(path) => path,
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
