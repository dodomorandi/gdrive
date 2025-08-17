use crate::common::delegate::BackoffConfig;
use crate::common::delegate::ChunkSize;
use crate::common::delegate::UploadDelegate;
use crate::common::delegate::UploadDelegateConfig;
use crate::common::file_helper;
use crate::common::file_info;
use crate::common::file_info::FileInfo;
use crate::common::hub_helper;
use crate::files;
use crate::files::info;
use crate::files::info::DisplayConfig;
use crate::hub::Hub;
use mime::Mime;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::time::Duration;

pub struct Config {
    pub file_id: String,
    pub file_path: Option<PathBuf>,
    pub mime_type: Option<Mime>,
    pub chunk_size: ChunkSize,
    pub print_chunk_errors: bool,
    pub print_chunk_info: bool,
}

pub async fn update(config: Config) -> Result<(), Error> {
    let hub = hub_helper::get_hub().await.map_err(Error::Hub)?;

    let delegate_config = UploadDelegateConfig {
        chunk_size: config.chunk_size,
        backoff_config: BackoffConfig {
            max_retries: 20,
            min_sleep: Duration::from_secs(1),
            max_sleep: Duration::from_secs(60),
        },
        print_chunk_errors: config.print_chunk_errors,
        print_chunk_info: config.print_chunk_info,
    };

    let mut file_helper = match file_helper::open_file(&config.file_path) {
        Ok(file_helper) => file_helper,
        Err(err) => {
            return Err(Error::OpenFile(
                config.file_path.unwrap_or_else(|| PathBuf::from("<stdin>")),
                err,
            ))
        }
    };

    let drive_file = info::get_file(&hub, &config.file_id)
        .await
        .map_err(Error::GetFile)?;

    let (file, file_path) = file_helper.file_mut_and_path();

    let file_info_config = file_info::Config {
        file_path,
        mime_type: config.mime_type.as_ref(),
        parents: drive_file.parents.clone(),
    };

    let file_info = match FileInfo::from_file(file, file_info_config) {
        Ok(file_info) => file_info,
        Err(source) => {
            return Err(Error::FileInfo {
                path: file_helper.into_path_buf(),
                source,
            })
        }
    };

    let reader = std::io::BufReader::new(file);

    println!("Updating {} with {}", config.file_id, file_path.display());

    let file = update_file(&hub, reader, &config.file_id, file_info, delegate_config)
        .await
        .map_err(Error::Update)?;

    println!("File successfully updated");

    let fields = files::info::prepare_fields(&file, &DisplayConfig::default());
    files::info::print_fields(&fields);

    Ok(())
}

pub async fn update_file<RS>(
    hub: &Hub,
    src_file: RS,
    file_id: &str,
    file_info: FileInfo<'_>,
    delegate_config: UploadDelegateConfig,
) -> Result<google_drive3::api::File, google_drive3::Error>
where
    RS: google_drive3::client::ReadSeek,
{
    let dst_file = google_drive3::api::File {
        name: Some(file_info.name.into_owned()),
        ..google_drive3::api::File::default()
    };

    let mut delegate = UploadDelegate::new(delegate_config);

    let req = hub
        .files()
        .update(dst_file, file_id)
        .param("fields", "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,description,webContentLink,webViewLink")
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true);

    let (_, file) = if file_info.size > 0 {
        req.upload_resumable(src_file, file_info.mime_type.into_owned())
            .await?
    } else {
        req.upload(src_file, file_info.mime_type.into_owned())
            .await?
    };

    Ok(file)
}

pub async fn update_metadata(
    hub: &Hub,
    delegate_config: UploadDelegateConfig,
    patch_file: PatchFile,
) -> Result<google_drive3::api::File, google_drive3::Error> {
    let mut delegate = UploadDelegate::new(delegate_config);

    let (_, file) = hub
        .files()
        .update(patch_file.file, &patch_file.id)
        .param("fields", "id,name,size,createdTime,modifiedTime,md5Checksum,mimeType,parents,shared,description,webContentLink,webViewLink")
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true)
        .doit_without_upload().await?;

    Ok(file)
}

#[derive(Debug)]
pub enum Error {
    Hub(hub_helper::Error),
    FileInfo {
        path: PathBuf,
        source: file_info::FromFileError,
    },
    OpenFile(PathBuf, file_helper::OpenFileError),
    GetFile(google_drive3::Error),
    Update(google_drive3::Error),
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
            Error::FileInfo { path, source: _ } => {
                write!(f, "unable to get file info for '{}'", path.display())
            }
            Error::OpenFile(path, err) => {
                write!(f, "Failed to open file '{}': {}", path.display(), err)
            }
            Error::GetFile(err) => write!(f, "Failed to get file: {err}"),
            Error::Update(err) => write!(f, "Failed to update file: {err}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatchFile {
    id: String,
    file: google_drive3::api::File,
}

impl PatchFile {
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            id,
            file: google_drive3::api::File::default(),
        }
    }

    #[must_use]
    pub fn with_name(&self, name: &str) -> Self {
        Self {
            file: google_drive3::api::File {
                name: Some(name.to_string()),
                ..self.file.clone()
            },
            ..self.clone()
        }
    }

    #[must_use]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[must_use]
    pub fn file(&self) -> google_drive3::api::File {
        self.file.clone()
    }
}
