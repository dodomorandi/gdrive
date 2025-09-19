use std::{
    error,
    fmt::{Display, Formatter},
    ops::Not,
    path::PathBuf,
};

use mime::Mime;

use crate::{
    common::{
        drive_file::{DocType, FileExtension},
        hub_helper::{get_hub, GetHubError},
        parse_md5_digest,
    },
    files,
    hub::Hub,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub file_id: String,
    pub file_path: PathBuf,
    pub existing_file_action: ExistingFileAction,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExistingFileAction {
    Abort,
    Overwrite,
}

pub async fn export(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;

    if config.file_path.exists() && config.existing_file_action == ExistingFileAction::Abort {
        return Err(Error::FileExists(config.file_path));
    }

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    let drive_mime = file.mime_type.ok_or(Error::MissingDriveMime)?;
    let Some(doc_type) = DocType::from_mime_type(&drive_mime) else {
        return Err(Error::UnsupportedDriveMime(drive_mime));
    };

    let extension = FileExtension::from_path(&config.file_path)
        .ok_or(Error::UnsupportedExportExtension(doc_type))?;

    if doc_type.can_export_to(extension).not() {
        return Err(Error::UnsupportedExportExtension(doc_type));
    }

    let mime_type = extension.get_export_mime();

    let body = export_file(&hub, &config.file_id, mime_type)
        .await
        .map_err(|err| Error::ExportFile(Box::new(err)))?;

    println!(
        "Exporting {} '{}' to {}",
        doc_type,
        file.name.unwrap_or_default(),
        config.file_path.display()
    );

    let md5_checksum = file.md5_checksum.as_deref().and_then(parse_md5_digest);
    files::download::save_body_to_file(body, &config.file_path, md5_checksum.as_ref())
        .await
        .map_err(Error::SaveFile)?;

    println!("Successfully exported {}", config.file_path.display());

    Ok(())
}

pub async fn export_file(
    hub: &Hub,
    file_id: &str,
    mime_type: &Mime,
) -> Result<hyper::Body, google_drive3::Error> {
    let response = hub
        .files()
        .export(file_id, mime_type.as_ref())
        .add_scope(google_drive3::api::Scope::Full)
        .doit()
        .await?;

    Ok(response.into_body())
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    FileExists(PathBuf),
    GetFile(Box<google_drive3::Error>),
    ExportFile(Box<google_drive3::Error>),
    MissingDriveMime,
    UnsupportedDriveMime(String),
    UnsupportedExportExtension(DocType),
    SaveFile(files::download::errors::SaveBodyToFile),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::FileExists(path) => {
                write!(
                    f,
                    "file '{}' already exists, use --overwrite to overwrite it",
                    path.display()
                )
            }
            Error::GetFile(_) => f.write_str("unable to get file"),
            Error::ExportFile(_) => f.write_str("unable to export file"),
            Error::MissingDriveMime => f.write_str("drive file does not have a mime type"),
            Error::UnsupportedDriveMime(mime) => {
                write!(f, "mime type on drive file '{mime}' is not supported")
            }
            Error::UnsupportedExportExtension(doc_type) => {
                write!(
                    f,
                    "export of a {doc_type} to this file type is not supported, supported file \
                    types are: "
                )?;

                let mut types = doc_type.supported_export_types().iter();
                if let Some(ty) = types.next() {
                    write!(f, "{ty}")?;
                    for ty in types {
                        write!(f, ", {ty}")?;
                    }
                } else {
                    f.write_str("[NONE]")?;
                }
                Ok(())
            }
            Error::SaveFile(_) => {
                write!(f, "failed to save file")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::FileExists(_)
            | Error::MissingDriveMime
            | Error::UnsupportedDriveMime(_)
            | Error::UnsupportedExportExtension(_)
            | Error::SaveFile(_) => None,
            Error::GetFile(source) | Error::ExportFile(source) => Some(source),
        }
    }
}
