use std::{error::Error, fmt::Display, io, path::PathBuf};

use md5::Digest;

use crate::common::{
    file_tree_drive::{self, errors::FileIdentifier},
    hub_helper::GetHubError,
};

#[derive(Debug)]
pub enum Download {
    Hub(GetHubError),
    GetFile(Box<google_drive3::Error>),
    #[expect(
        clippy::enum_variant_names,
        reason = "this is the actual download operation"
    )]
    DownloadFile(Box<google_drive3::Error>),
    MissingFileName(FileIdentifier),
    FileExists(FileIdentifier),
    IsDirectory(FileIdentifier),
    CreateDirectory(PathBuf, io::Error),
    CopyFile(io::Error),
    RenameFile(io::Error),
    CreateFileTree(file_tree_drive::errors::FileTreeDrive),
    DestinationPathDoesNotExist(PathBuf),
    DestinationPathNotADirectory(PathBuf),
    CanonicalizeDestinationPath(PathBuf, io::Error),
    MissingShortcutTarget(FileIdentifier),
    IsShortcut(FileIdentifier),
    StdoutNotValidDestination,
    SaveBodyToStdout(SaveBodyToStdout),
    SaveBodyToFile {
        path: PathBuf,
        source: SaveBodyToFile,
    },
}

impl Display for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Download::Hub(_) => f.write_str("unable to get drive hub"),
            Download::GetFile(_) => f.write_str("unable to get file info"),
            Download::DownloadFile(_) => f.write_str("unable to download file from drive"),
            Download::MissingFileName(identifier) => {
                write!(f, "file{} does not have a name", identifier.display())
            }
            Download::FileExists(identifier) => write!(
                f,
                "file{} already exists, use --overwrite to overwrite it",
                identifier.display()
            ),
            Download::IsDirectory(identifier) => write!(
                f,
                "file{} is a directory, use --recursive to download directories",
                identifier.display()
            ),
            Download::CreateDirectory(path, _) => {
                write!(f, "unable to create directory '{}'", path.display())
            }
            Download::CopyFile(_) => f.write_str("unable to copy file"),
            Download::RenameFile(_) => f.write_str("unable to rename file"),
            Download::CreateFileTree(_) => f.write_str("unable to create file tree"),
            Download::DestinationPathDoesNotExist(path) => {
                write!(f, "destination path '{}' does not exist", path.display())
            }
            Download::DestinationPathNotADirectory(path) => write!(
                f,
                "destination path '{}' is not a directory",
                path.display()
            ),
            Download::CanonicalizeDestinationPath(path, _) => write!(
                f,
                "unable to canoicalize destination path '{}'",
                path.display()
            ),
            Download::MissingShortcutTarget(identifier) => {
                write!(f, "shortcut{} does not have a target", identifier.display())
            }
            Download::IsShortcut(identifier) => write!(
                f,
                "file{} is a shortcut, use --follow-shortcuts to download the file it points to",
                identifier.display()
            ),
            Download::StdoutNotValidDestination => {
                f.write_str("stdout is not a valid destination for this combination of options")
            }
            Download::SaveBodyToStdout(_) => f.write_str("unabl to save body to stdout"),
            Download::SaveBodyToFile { path, source: _ } => {
                write!(f, "unable to save body to file '{}'", path.display())
            }
        }
    }
}

impl Error for Download {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Download::Hub(get_hub_error) => Some(get_hub_error),
            Download::GetFile(error) | Download::DownloadFile(error) => Some(error),
            Download::MissingFileName(_)
            | Download::FileExists(_)
            | Download::IsDirectory(_)
            | Download::CreateFileTree(_)
            | Download::DestinationPathDoesNotExist(_)
            | Download::DestinationPathNotADirectory(_)
            | Download::MissingShortcutTarget(_)
            | Download::IsShortcut(_)
            | Download::StdoutNotValidDestination => None,
            Download::CreateDirectory(_, source)
            | Download::CanonicalizeDestinationPath(_, source) => Some(source),
            Download::CopyFile(error) | Download::RenameFile(error) => Some(error),
            Download::SaveBodyToStdout(save_body_to_stdout) => Some(save_body_to_stdout),
            Download::SaveBodyToFile { source, .. } => Some(source),
        }
    }
}

impl From<SaveBodyToStdout> for Download {
    fn from(value: SaveBodyToStdout) -> Self {
        Download::SaveBodyToStdout(value)
    }
}

#[derive(Debug)]
pub enum SaveBodyToStdout {
    ReadChunk(hyper::Error),
    WriteChunk(io::Error),
}

impl Display for SaveBodyToStdout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let operation = match self {
            SaveBodyToStdout::ReadChunk(_) => "read",
            SaveBodyToStdout::WriteChunk(_) => "write",
        };

        write!(f, "unable to {operation} chunk of bytes")
    }
}

impl Error for SaveBodyToStdout {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SaveBodyToStdout::ReadChunk(source) => Some(source),
            SaveBodyToStdout::WriteChunk(source) => Some(source),
        }
    }
}

#[derive(Debug)]
pub enum SaveBodyToFile {
    CreateFile(io::Error),
    ReadChunk(hyper::Error),
    WriteChunk(io::Error),
    Md5Mismatch { expected: Digest, actual: Digest },
    RenameFile(io::Error),
}

impl Display for SaveBodyToFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveBodyToFile::CreateFile(_) => f.write_str("unable to create file"),
            SaveBodyToFile::ReadChunk(_) => f.write_str("unable to read chunk of bytes"),
            SaveBodyToFile::WriteChunk(_) => f.write_str("unable to write chunk of bytes"),
            SaveBodyToFile::Md5Mismatch { expected, actual } => {
                write!(
                    f,
                    "md5 mismatches (expected {expected:x}, actual is {actual:x})"
                )
            }
            SaveBodyToFile::RenameFile(_) => f.write_str("unable to rename file"),
        }
    }
}

impl Error for SaveBodyToFile {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SaveBodyToFile::ReadChunk(source) => Some(source),
            SaveBodyToFile::Md5Mismatch { .. } => None,
            SaveBodyToFile::CreateFile(source)
            | SaveBodyToFile::WriteChunk(source)
            | SaveBodyToFile::RenameFile(source) => Some(source),
        }
    }
}
