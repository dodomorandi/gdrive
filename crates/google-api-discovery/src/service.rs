//! URL helpers for the Google API Discovery Service.

use std::{
    borrow::Cow,
    fmt::{self, Write as _},
};

/// The default base URL for the Google API Discovery Service directory.
pub const DEFAULT_BASE_URL: &str = "https://discovery.googleapis.com/discovery/v1";

/// The default base URL for individual REST discovery documents.
pub const DEFAULT_DOCUMENT_BASE_URL: &str = "https://www.googleapis.com/discovery/v1";

/// Parameters accepted by `discovery.apis.list`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ListApisParams<'a> {
    /// Include only APIs with this name. An empty name omits the filter.
    pub name: Cow<'a, str>,

    /// Return only the preferred version of each API.
    /// False is omitted because it is the service default.
    pub preferred: bool,
}

impl<'a> ListApisParams<'a> {
    /// Creates parameters with the service's default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: Cow::Borrowed(""),
            preferred: false,
        }
    }

    /// Sets the API name filter.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the preferred-version filter.
    #[must_use]
    pub const fn with_preferred(mut self, preferred: bool) -> Self {
        self.preferred = preferred;
        self
    }
}

/// Builds URLs for the Google API Discovery Service.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryService<'a> {
    /// Base URL for the APIs directory.
    pub base_url: Cow<'a, str>,

    /// Base URL for individual REST discovery documents.
    pub document_base_url: Cow<'a, str>,
}

impl Default for DiscoveryService<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DiscoveryService<'a> {
    /// Creates a URL builder using Google's public Discovery Service endpoints.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_url: Cow::Borrowed(DEFAULT_BASE_URL),
            document_base_url: Cow::Borrowed(DEFAULT_DOCUMENT_BASE_URL),
        }
    }

    /// Sets the base URL for the APIs directory.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<Cow<'a, str>>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the base URL for individual REST discovery documents.
    #[must_use]
    pub fn with_document_base_url(mut self, document_base_url: impl Into<Cow<'a, str>>) -> Self {
        self.document_base_url = document_base_url.into();
        self
    }

    /// Builds the URL for `discovery.apis.list`.
    #[must_use]
    pub fn list_apis_url(&self, params: &ListApisParams<'_>) -> String {
        let base_url = self.base_url.trim_end_matches('/');
        let mut url = format!("{base_url}/apis");
        append_query(&mut url, params);
        url
    }

    /// Builds the URL for `discovery.apis.getRest`.
    #[must_use]
    pub fn get_rest_url(&self, api: &str, version: &str) -> String {
        let base_url = self.document_base_url.trim_end_matches('/');
        format!(
            "{base_url}/apis/{}/{}/rest",
            PercentEncoded(api),
            PercentEncoded(version)
        )
    }
}

fn append_query(url: &mut String, params: &ListApisParams<'_>) {
    let mut has_query = false;

    if !params.name.is_empty() {
        write!(url, "?name={}", PercentEncoded(params.name.as_ref()))
            .expect("writing to a String cannot fail");
        has_query = true;
    }

    if params.preferred {
        if has_query {
            url.push('&');
        }
        url.push_str("preferred=true");
    }
}

struct PercentEncoded<'value>(&'value str);

impl fmt::Display for PercentEncoded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.bytes() {
            if is_unreserved(byte) {
                f.write_char(char::from(byte))?;
            } else {
                write!(f, "%{byte:02X?}")?;
            }
        }
        Ok(())
    }
}

const fn is_unreserved(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
    )
}
