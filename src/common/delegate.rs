use bytesize::ByteSize;
use error_trace::ErrorTrace;
use google_drive3::hyper;
use google_drive3::hyper::http;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct UploadDelegateConfig {
    pub chunk_size: ChunkSize,
    pub backoff_config: BackoffConfig,
    pub print_chunk_errors: bool,
    pub print_chunk_info: bool,
}

pub struct UploadDelegate {
    config: UploadDelegateConfig,
    backoff: Backoff,
    resumable_upload_url: Option<String>,
    previous_chunk: Option<google_drive3::client::ContentRange>,
}

impl UploadDelegate {
    #[must_use]
    pub fn new(config: UploadDelegateConfig) -> UploadDelegate {
        let backoff = Backoff::new(&config.backoff_config);

        UploadDelegate {
            config,
            backoff,
            resumable_upload_url: None,
            previous_chunk: None,
        }
    }

    fn print_chunk_info(&self, chunk: &google_drive3::client::ContentRange) {
        if self.config.print_chunk_info {
            if let Some(range) = &chunk.range {
                let chunk_size = if range.last < u64::MAX {
                    (range.last + 1).saturating_sub(range.first)
                } else {
                    (range.last - range.first).saturating_sub(1)
                };

                let action = if Some(chunk) == self.previous_chunk.as_ref() {
                    "Retrying"
                } else {
                    "Uploading"
                };

                println!(
                    "Info: {} {} chunk ({}-{} of {})",
                    action,
                    ByteSize::b(chunk_size).display().si(),
                    range.first,
                    range.last,
                    chunk.total_length
                );
            }
        }
    }
}

impl google_drive3::client::Delegate for UploadDelegate {
    fn chunk_size(&mut self) -> u64 {
        self.config.chunk_size.in_bytes()
    }

    fn cancel_chunk_upload(&mut self, chunk: &google_drive3::client::ContentRange) -> bool {
        self.print_chunk_info(chunk);
        self.previous_chunk = Some(chunk.clone());

        false
    }

    fn store_upload_url(&mut self, url: Option<&str>) {
        self.resumable_upload_url = url.map(ToString::to_string);
    }

    fn upload_url(&mut self) -> Option<String> {
        self.resumable_upload_url.clone()
    }

    fn http_error(&mut self, err: &hyper::Error) -> google_drive3::client::Retry {
        if self.config.print_chunk_errors {
            eprintln!("Warning: Failed attempt to upload chunk: {}", err.trace());
        }
        self.backoff.retry()
    }

    fn http_failure(
        &mut self,
        res: &http::response::Response<hyper::body::Body>,
        _err: Option<serde_json::Value>,
    ) -> google_drive3::client::Retry {
        let status = res.status();

        if should_retry(status) {
            if self.config.print_chunk_errors {
                eprintln!(
                    "Warning: Failed attempt to upload chunk. Status code: {}, body: {:?}",
                    status,
                    res.body()
                );
            }
            self.backoff.retry()
        } else {
            google_drive3::client::Retry::Abort
        }
    }
}

fn should_retry(status: http::StatusCode) -> bool {
    status.is_server_error() || status == http::StatusCode::TOO_MANY_REQUESTS
}

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub max_retries: u32,
    pub min_sleep: Duration,
    pub max_sleep: Duration,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        BackoffConfig {
            max_retries: 100,
            min_sleep: Duration::from_secs(1),
            max_sleep: Duration::from_secs(60),
        }
    }
}

pub struct Backoff {
    attempts: u32,
    backoff: exponential_backoff::Backoff,
}

impl Backoff {
    #[must_use]
    pub fn new(config: &BackoffConfig) -> Backoff {
        Backoff {
            attempts: 0,
            backoff: exponential_backoff::Backoff::new(
                config.max_retries,
                config.min_sleep,
                config.max_sleep,
            ),
        }
    }

    fn retry(&mut self) -> google_drive3::client::Retry {
        self.attempts += 1;
        self.backoff.next(self.attempts).map_or(
            google_drive3::client::Retry::Abort,
            google_drive3::client::Retry::After,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub enum ChunkSize {
    Approx1,
    Approx2,
    Approx4,
    Approx8,
    Approx16,
    #[default]
    Approx32,
    Approx64,
    Approx128,
    Approx256,
    Approx512,
    Approx1024,
    Approx2048,
    Approx4096,
    Approx8192,
}

impl ChunkSize {
    #[must_use]
    pub fn in_bytes(&self) -> u64 {
        let exponent = match self {
            ChunkSize::Approx1 => 20,
            ChunkSize::Approx2 => 21,
            ChunkSize::Approx4 => 22,
            ChunkSize::Approx8 => 23,
            ChunkSize::Approx16 => 24,
            ChunkSize::Approx32 => 25,
            ChunkSize::Approx64 => 26,
            ChunkSize::Approx128 => 27,
            ChunkSize::Approx256 => 28,
            ChunkSize::Approx512 => 29,
            ChunkSize::Approx1024 => 30,
            ChunkSize::Approx2048 => 31,
            ChunkSize::Approx4096 => 32,
            ChunkSize::Approx8192 => 33,
        };

        u64::pow(2, exponent)
    }
}

impl FromStr for ChunkSize {
    type Err = InvalidChunkSize;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(ChunkSize::Approx1),
            "2" => Ok(ChunkSize::Approx2),
            "4" => Ok(ChunkSize::Approx4),
            "8" => Ok(ChunkSize::Approx8),
            "16" => Ok(ChunkSize::Approx16),
            "32" => Ok(ChunkSize::Approx32),
            "64" => Ok(ChunkSize::Approx64),
            "128" => Ok(ChunkSize::Approx128),
            "256" => Ok(ChunkSize::Approx256),
            "512" => Ok(ChunkSize::Approx512),
            "1024" => Ok(ChunkSize::Approx1024),
            "2048" => Ok(ChunkSize::Approx2048),
            "4096" => Ok(ChunkSize::Approx4096),
            "8192" => Ok(ChunkSize::Approx8192),
            _ => Err(InvalidChunkSize),
        }
    }
}

impl Display for ChunkSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkSize::Approx1 => write!(f, "1"),
            ChunkSize::Approx2 => write!(f, "2"),
            ChunkSize::Approx4 => write!(f, "4"),
            ChunkSize::Approx8 => write!(f, "8"),
            ChunkSize::Approx16 => write!(f, "16"),
            ChunkSize::Approx32 => write!(f, "32"),
            ChunkSize::Approx64 => write!(f, "64"),
            ChunkSize::Approx128 => write!(f, "128"),
            ChunkSize::Approx256 => write!(f, "256"),
            ChunkSize::Approx512 => write!(f, "512"),
            ChunkSize::Approx1024 => write!(f, "1024"),
            ChunkSize::Approx2048 => write!(f, "2048"),
            ChunkSize::Approx4096 => write!(f, "4096"),
            ChunkSize::Approx8192 => write!(f, "8192"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InvalidChunkSize;

impl Display for InvalidChunkSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a valid chunk size, must be a power of 2 between 1 and 8192")
    }
}

impl std::error::Error for InvalidChunkSize {}
