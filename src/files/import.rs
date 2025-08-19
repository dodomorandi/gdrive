use std::{
    error,
    fmt::{Display, Formatter},
    fs, io,
    path::PathBuf,
};

use crate::{
    common::{
        delegate::UploadDelegateConfig,
        drive_file::{self, DocType},
        file_info::{self, FileInfo},
        hub_helper::{self, GetHubError},
    },
    files::{self, info::DisplayConfig},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub file_path: PathBuf,
    pub parents: Option<Vec<String>>,
    pub print_only_id: bool,
}

pub async fn import(config: Config) -> Result<(), Error> {
    let hub = hub_helper::get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let doc_type =
        drive_file::DocType::from_file_path(&config.file_path).ok_or(Error::UnsupportedFileType)?;
    let mime_type = doc_type.mime();

    let file = fs::File::open(&config.file_path)
        .map_err(|err| Error::OpenFile(config.file_path.clone(), err))?;

    let file_info = match FileInfo::from_file(
        &file,
        file_info::Config {
            file_path: &config.file_path,
            mime_type: Some(mime_type),
            parents: config.parents.clone(),
        },
    ) {
        Ok(file_info) => file_info,
        Err(source) => {
            return Err(Error::FileInfo {
                path: config.file_path,
                source,
            })
        }
    };

    let reader = std::io::BufReader::new(file);

    if !config.print_only_id {
        println!("Importing {} as a {}", config.file_path.display(), doc_type);
    }

    let file = files::upload::upload_file(&hub, reader, None, file_info, &delegate_config)
        .await
        .map_err(Error::UploadFile)?;

    if config.print_only_id {
        print!("{}", file.id.unwrap_or_default());
    } else {
        println!("File successfully imported");
        let fields = files::info::prepare_fields(&file, &DisplayConfig::default());
        files::info::print_fields(&fields);
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    OpenFile(PathBuf, io::Error),
    FileInfo {
        path: PathBuf,
        source: file_info::FromFileError,
    },
    UploadFile(google_drive3::Error),
    UnsupportedFileType,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::FileInfo { source, .. } => Some(source),
            // FIXME: correctly impl std::error::Error
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(err) => write!(f, "{err}"),
            Error::OpenFile(path, err) => {
                write!(f, "Failed to open file '{}': {}", path.display(), err)
            }
            Error::FileInfo { path, source: _ } => {
                write!(f, "unable to get file info for '{}'", path.display())
            }
            Error::UploadFile(err) => {
                write!(f, "Failed to upload file: {err}")
            }
            Error::UnsupportedFileType => {
                const _: () = const {
                    assert!(
                        !DocType::SUPPORTED_INPUT_TYPES.is_empty(),
                        "DocType::SUPPORTED_INPUT_TYPES should not be empty"
                    );
                };

                f.write_str("Unsupported file type, supported file types: ")?;
                let mut supported_types = DocType::SUPPORTED_INPUT_TYPES.iter();
                write!(f, "{}", supported_types.next().unwrap())?;
                for supported_type in supported_types {
                    write!(f, ", {supported_type}")?;
                }
                Ok(())
            }
        }
    }
}
