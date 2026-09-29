//! Fixed resource-kind values used by Discovery documents.

use serde::{Deserialize, Serialize};

/// Identifies a Discovery APIs directory list response.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum DirectoryList {
    /// The `discovery#directoryList` resource kind.
    #[default]
    #[serde(rename = "discovery#directoryList")]
    DirectoryList,
}

impl DirectoryList {
    /// Returns the JSON representation of the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "discovery#directoryList"
    }
}

/// Identifies an item in a Discovery APIs directory list.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum DirectoryItem {
    /// The `discovery#directoryItem` resource kind.
    #[default]
    #[serde(rename = "discovery#directoryItem")]
    DirectoryItem,
}

impl DirectoryItem {
    /// Returns the JSON representation of the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "discovery#directoryItem"
    }
}

/// Identifies a REST discovery document.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RestDescription {
    /// The `discovery#restDescription` resource kind.
    #[default]
    #[serde(rename = "discovery#restDescription")]
    RestDescription,
}

impl RestDescription {
    /// Returns the JSON representation of the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "discovery#restDescription"
    }
}
