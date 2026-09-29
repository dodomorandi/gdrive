//! Serde models and URL helpers for the Google API Discovery Service.
//!
//! Deserialize response bodies directly into [`DirectoryList`] or [`RestDescription`] with
//! `serde_json`. URL helpers produce strings for the caller's HTTP client; this crate never opens
//! sockets or chooses an asynchronous runtime.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod kind;
pub mod models;
pub mod service;

pub use models::{DirectoryItem, DirectoryList, RestDescription};
pub use service::{DiscoveryService, ListApisParams};
