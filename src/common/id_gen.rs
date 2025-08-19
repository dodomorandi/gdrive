use std::{
    error,
    fmt::{Display, Formatter},
};

use crate::{common::delegate::UploadDelegateConfig, files::generate_ids, hub::Hub};

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

    pub async fn next(&mut self) -> Result<String, NextError> {
        if let Some(id) = self.ids.pop() {
            Ok(id)
        } else {
            self.ids = generate_ids::generate_ids(self.hub, 1000, self.delegate_config)
                .await
                .map_err(|err| NextError::GenerateIds(Box::new(err)))?;
            let id = self.ids.pop().ok_or(NextError::OutOfIds)?;
            Ok(id)
        }
    }
}

#[derive(Debug)]
pub enum NextError {
    // TODO: remove this allocation
    GenerateIds(Box<google_drive3::Error>),
    OutOfIds,
}

impl error::Error for NextError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NextError::GenerateIds(source) => Some(source),
            NextError::OutOfIds => None,
        }
    }
}

impl Display for NextError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NextError::GenerateIds(_) => "failed generate drive identifiers",
            NextError::OutOfIds => "no more identifiers available",
        };

        f.write_str(s)
    }
}
