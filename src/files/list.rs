use std::{
    cmp::min,
    error,
    fmt::{self, Display, Formatter},
    io,
    str::FromStr,
};

use crate::{
    common::{
        drive_file,
        hub_helper::{get_hub, GetHubError},
        table::{self, Table},
    },
    files::{self, info::DisplayConfig},
    hub::Hub,
};

const MAX_PAGE_SIZE: usize = 1000;

pub struct Config {
    pub query: ListQuery,
    pub order_by: ListSortOrder,
    pub max_files: usize,
    pub skip_header: bool,
    pub truncate_name: bool,
    pub field_separator: String,
}

pub async fn list(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;
    let files = list_files(
        &hub,
        ListFilesConfig {
            query: &config.query,
            order_by: &config.order_by,
            max_files: config.max_files,
        },
    )
    .await?;

    let mut values: Vec<[String; 5]> = vec![];

    for file in files {
        let file_type = simplified_file_type(&file);
        let file_name = format_file_name(&config, &file);

        values.push([
            file.id.unwrap_or_default(),
            file_name,
            file_type,
            file.size
                .map(|bytes| {
                    files::info::DisplayBytes {
                        bytes: bytes.try_into().unwrap_or(0),
                        config: &DisplayConfig::default(),
                    }
                    .to_string()
                })
                .unwrap_or_default(),
            file.created_time
                .map(|created_time| files::info::format_date_time(created_time).to_string())
                .unwrap_or_default(),
        ]);
    }

    let table = Table {
        header: ["Id", "Name", "Type", "Size", "Created"],
        values,
    };

    let _ = table::write(
        io::stdout(),
        table,
        &table::DisplayConfig {
            skip_header: config.skip_header,
            separator: &config.field_separator,
        },
    );

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct ListFilesConfig<'a> {
    pub query: &'a ListQuery,
    pub order_by: &'a ListSortOrder,
    pub max_files: usize,
}

pub async fn list_files(
    hub: &Hub,
    config: ListFilesConfig<'_>,
) -> Result<Vec<google_drive3::api::File>, Error> {
    let mut collected_files: Vec<google_drive3::api::File> = vec![];
    let mut next_page_token: Option<String> = None;

    loop {
        let max_files = config.max_files - collected_files.len();
        let page_size = min(MAX_PAGE_SIZE, max_files);

        let mut req = hub.files().list();

        if let Some(token) = next_page_token {
            req = req.page_token(&token);
        }

        let (_, file_list) = req
            .page_size(page_size.try_into().unwrap_or(i32::MAX))
            .q(&config.query.to_string())
            .order_by(&config.order_by.to_string())
            .add_scope(google_drive3::api::Scope::Full)
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
            .param(
                "fields",
                "files(id,name,md5Checksum,mimeType,size,createdTime,parents),nextPageToken",
            )
            .doit()
            .await
            .map_err(|err| Error::ListFiles(Box::new(err)))?;

        if let Some(mut files) = file_list.files {
            collected_files.append(&mut files);
        }

        next_page_token = file_list.next_page_token;

        if collected_files.len() >= config.max_files || next_page_token.is_none() {
            break;
        }
    }

    collected_files.truncate(config.max_files);
    Ok(collected_files)
}

#[derive(Debug, Clone, Default)]
pub enum ListQuery {
    #[default]
    RootNotTrashed,
    FilesOnDrive {
        drive_id: String,
    },
    FilesInFolder {
        folder_id: String,
    },
    Custom(String),
    None,
}

impl FromStr for ListQuery {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(ListQuery::None)
        } else {
            Ok(ListQuery::Custom(s.to_string()))
        }
    }
}

impl Display for ListQuery {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ListQuery::RootNotTrashed => {
                write!(f, "'root' in parents and trashed = false")
            }

            ListQuery::FilesOnDrive { drive_id } => {
                write!(f, "'{drive_id}' in parents and trashed = false")
            }

            ListQuery::FilesInFolder { folder_id } => {
                write!(f, "'{folder_id}' in parents and trashed = false")
            }

            ListQuery::Custom(query) => {
                write!(f, "{query}")
            }

            ListQuery::None => {
                write!(f, "")
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ListSortOrder {
    #[default]
    FolderModifiedName,
    Custom(String),
}

impl FromStr for ListSortOrder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("Sort by can't be an empty string".to_string())
        } else {
            Ok(ListSortOrder::Custom(s.to_string()))
        }
    }
}

impl fmt::Display for ListSortOrder {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ListSortOrder::FolderModifiedName => {
                write!(f, "folder,modifiedTime desc,name")
            }

            ListSortOrder::Custom(query) => {
                write!(f, "{query}")
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Hub(GetHubError),
    ListFiles(Box<google_drive3::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::ListFiles(_) => f.write_str("unable to list files"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::ListFiles(source) => Some(source),
        }
    }
}

fn simplified_file_type(file: &google_drive3::api::File) -> String {
    if drive_file::is_directory(file) {
        String::from("folder")
    } else if drive_file::is_binary(file) {
        String::from("regular")
    } else if drive_file::is_shortcut(file) {
        String::from("shortcut")
    } else {
        String::from("document")
    }
}

fn format_file_name(config: &Config, file: &google_drive3::api::File) -> String {
    let file_name = file.name.as_ref();

    if config.truncate_name {
        file_name
            .map(|s| truncate_middle(s, 41))
            .unwrap_or_default()
    } else {
        file_name.map(ToString::to_string).unwrap_or_default()
    }
}

// Truncates string to given max length, and inserts ellipsis into
// the middle of the string to signify that the string has been truncated
fn truncate_middle(s: &str, max_length: usize) -> String {
    let chars: Vec<char> = s.chars().collect();

    if chars.len() <= max_length {
        return s.to_string();
    }

    let tail_count = max_length / 2;
    let head_count = max_length - tail_count - 1;

    let head: String = chars[0..head_count].iter().collect();
    let tail: String = chars[chars.len() - tail_count..].iter().collect();

    [head, tail].join("…")
}
