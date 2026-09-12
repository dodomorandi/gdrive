use std::{
    borrow::Cow,
    error,
    fmt::{Display, Formatter},
    fs,
    path::Path,
};

pub(crate) struct FileInfo<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) mime_type: Cow<'a, mime::Mime>,
    pub(crate) parents: Option<Vec<String>>,
    pub(crate) size: u64,
}

pub(crate) struct Config<'a> {
    pub(crate) file_path: &'a Path,
    pub(crate) mime_type: Option<&'a mime::Mime>,
    pub(crate) parents: Option<Vec<String>>,
}

impl<'a> FileInfo<'a> {
    pub(crate) fn from_file(file: &fs::File, config: Config<'a>) -> Result<Self, FromFileError> {
        let file_name = config
            .file_path
            .file_name()
            .map(|s| s.to_string_lossy())
            .ok_or(FromFileError)?;

        let file_size = file.metadata().map_or(0, |m| m.len());

        let mime_type = config.mime_type.map_or_else(
            || {
                mime_guess::from_path(config.file_path)
                    .first()
                    .map_or(Cow::Borrowed(&mime::APPLICATION_OCTET_STREAM), Cow::Owned)
            },
            Cow::Borrowed,
        );

        Ok(FileInfo {
            name: file_name,
            mime_type,
            parents: config.parents,
            size: file_size,
        })
    }
}

#[derive(Debug)]
pub(crate) struct FromFileError;

impl error::Error for FromFileError {}

impl Display for FromFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid file path")
    }
}
