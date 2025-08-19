use crate::common::delegate::UploadDelegateConfig;
use crate::files::generate_ids;
use crate::hub::Hub;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;

pub struct IdGen<'a> {
    hub: &'a Hub,
    delegate_config: &'a UploadDelegateConfig,
    ids: Vec<String>,
}

impl<'a> IdGen<'a> {
    #[must_use]
    pub fn new(hub: &'a Hub, delegate_config: &'a UploadDelegateConfig) -> Self {
        Self {
            hub,
            delegate_config,
            ids: Vec::new(),
        }
    }

    pub async fn next(&mut self) -> Result<String, Error> {
        if let Some(id) = self.ids.pop() {
            Ok(id)
        } else {
            self.ids = self.generate_ids().await?;
            let id = self.ids.pop().ok_or(Error::OutOfIds)?;
            Ok(id)
        }
    }

    async fn generate_ids(&self) -> Result<Vec<String>, Error> {
        generate_ids::generate_ids(self.hub, 1000, self.delegate_config)
            .await
            .map_err(|err| Error::GenerateIds(Box::new(err)))
    }
}

#[derive(Debug)]
pub enum Error {
    // TODO: remove this allocation
    GenerateIds(Box<google_drive3::Error>),
    OutOfIds,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::GenerateIds(source) => Some(source),
            Error::OutOfIds => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::GenerateIds(_) => "failed generate drive identifiers",
            Error::OutOfIds => "no more identifiers available",
        };

        f.write_str(s)
    }
}
