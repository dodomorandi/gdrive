use std::{
    error,
    fmt::{self, Display, Formatter},
};

use bytesize::ByteSize;
use google_drive3::chrono::{
    self,
    format::{DelayedFormat, StrftimeItems},
    DateTime,
};

use crate::{
    common::hub_helper::{get_hub, GetHubError},
    hub::Hub,
};

pub struct Config {
    pub file_id: String,
    pub size_in_bytes: bool,
}

pub async fn info(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;

    let file = get_file(&hub, &config.file_id)
        .await
        .map_err(Error::GetFile)?;

    print_file_info(
        &file,
        &DisplayConfig {
            size_in_bytes: config.size_in_bytes,
        },
    );

    Ok(())
}

pub async fn get_file(
    hub: &Hub,
    file_id: &str,
) -> Result<google_drive3::api::File, google_drive3::Error> {
    let (_, file) = hub
        .files()
        .get(file_id)
        .param(
            "fields",
            "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,\
            description,webContentLink,webViewLink,shortcutDetails(targetId,targetMimeType)",
        )
        .supports_all_drives(true)
        .add_scope(google_drive3::api::Scope::Full)
        .doit()
        .await?;

    Ok(file)
}

#[derive(Debug, Clone, Default)]
pub struct DisplayConfig {
    pub size_in_bytes: bool,
}

pub(crate) fn print_file_info(file: &google_drive3::api::File, display_config: &DisplayConfig) {
    let google_drive3::api::File {
        created_time,
        id,
        md5_checksum,
        mime_type,
        modified_time,
        name,
        parents,
        shared,
        size,
        web_view_link,
        ..
    } = file;

    print_field("Id", id.as_ref());
    print_field("Name", name.as_ref());
    print_field("Mime", mime_type.as_ref());
    print_field(
        "Size",
        size.map(|bytes| DisplayBytes {
            bytes: u64::try_from(bytes).unwrap_or(0),
            config: display_config,
        }),
    );
    print_field("Created", created_time.map(format_date_time));
    print_field("Modified", modified_time.map(format_date_time));
    print_field("MD5", md5_checksum.as_ref());
    print_field("Shared", shared.map(format_bool));
    print_field("Parents", parents.as_deref().map(DisplayJoinedSlice));
    print_field("ViewUrl", web_view_link.as_ref());
}

fn print_field(name: &str, value: Option<impl Display>) {
    if let Some(value) = value {
        println!("{name}: {value}");
    }
}

// TODO: move to common
#[must_use]
pub fn format_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DisplayJoinedSlice<'a, T>(pub &'a [T]);

impl<T> Display for DisplayJoinedSlice<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
            for element in iter {
                write!(f, ", {element}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayBytes<'a> {
    pub bytes: u64,
    pub config: &'a DisplayConfig,
}

impl Display for DisplayBytes<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let &Self { bytes, config } = self;

        if config.size_in_bytes {
            write!(f, "{bytes}")
        } else {
            write!(f, "{}", ByteSize::b(bytes).display().si())
        }
    }
}

#[must_use]
pub fn format_date_time(utc_time: DateTime<chrono::Utc>) -> DelayedFormat<StrftimeItems<'static>> {
    let local_time = DateTime::<chrono::Local>::from(utc_time);
    local_time.format("%Y-%m-%d %H:%M:%S")
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    GetFile(google_drive3::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::GetFile(_) => f.write_str("unable to get file"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source) => Some(source),
        }
    }
}
