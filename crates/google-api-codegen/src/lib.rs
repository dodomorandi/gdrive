//! Generates parser-first, sans-I/O Rust modules from a Google Discovery document.
//!
//! The generator consumes the parser models from `google-api-discovery` and emits a hierarchical
//! [`Generated`] value. Its output types implement [`ToTokens`], so callers can render a complete
//! crate with `quote!(#generated)` or write each [`GeneratedFile`] separately. Generated crates
//! need `serde` with its `derive` feature and `serde_json` as dependencies. The generator never
//! opens sockets, chooses an async runtime, or depends on an HTTP client.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod attributes;
mod error;
mod identifier;
mod output;
mod resource;
mod schema;

use std::ops::Not;

use google_api_discovery::RestDescription;
use quote::quote;

pub use error::{GenerationError, IdentifierKind};
pub use output::{FileDeclarations, Generated, GeneratedFile, GeneratedModule};
pub use proc_macro2::TokenStream;
pub use quote::ToTokens;

/// Generates code for a Discovery document with the default output layout.
#[derive(Debug, Default, Clone, Copy)]
pub struct Generator;

impl Generator {
    /// Creates a generator with the default output layout.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Generates a hierarchical file and module tree for a Discovery document.
    ///
    /// The default paths are `metadata.rs`, `protocol.rs`, `schemas.rs`, `api.rs`, and
    /// `resources.rs`, relative to the generated crate's `src` directory.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::UnknownSchema`] when a `$ref` does not name a schema in the
    /// document, or [`GenerationError::DuplicateIdentifier`] when two API names map to one Rust
    /// identifier.
    pub fn generate(&self, description: &RestDescription) -> Result<Generated, GenerationError> {
        let mut schemas = schema::SchemaGenerator::new(description)?;
        let named_schema_items = schemas.prepare()?;
        let (api, resources) = resource::generate_resources(description, &mut schemas)?;
        let schemas = schemas.finish(named_schema_items);

        let mut files = vec![
            GeneratedFile::new("metadata.rs", generate_metadata(description)),
            GeneratedFile::new("protocol.rs", generate_protocol()),
            GeneratedFile::new("schemas.rs", schemas),
        ];
        if api.is_empty().not() {
            files.push(GeneratedFile::new("api.rs", api));
        }
        if resources.is_empty().not() {
            files.push(GeneratedFile::new("resources.rs", resources));
        }
        Ok(Generated { files })
    }
}

/// Generates a hierarchical file and module tree for a Discovery document.
///
/// # Errors
///
/// Returns the same errors as [`Generator::generate`].
pub fn generate(description: &RestDescription) -> Result<Generated, GenerationError> {
    Generator::new().generate(description)
}

fn generate_metadata(description: &RestDescription) -> GeneratedModule {
    let mut module = GeneratedModule::new("metadata");
    let documentation = if description.description.trim().is_empty() {
        format!("Metadata for the `{}` API.", description.id)
    } else {
        description.description.clone()
    };
    module.set_documentation(documentation);
    let id = &description.id;
    let name = &description.name;
    let version = &description.version;
    let revision = &description.revision;
    let title = &description.title;
    let root_url = &description.root_url;
    let service_path = &description.service_path;
    let batch_path = &description.batch_path;
    let base_url = &description.base_url;

    module.add_items([
        quote! {
            #[doc = "The Discovery document identifier."]
            pub const API_ID: &str = #id;
        },
        quote! {
            #[doc = "The API name."]
            pub const API_NAME: &str = #name;
        },
        quote! {
            #[doc = "The API version."]
            pub const API_VERSION: &str = #version;
        },
        quote! {
            #[doc = "The Discovery document revision."]
            pub const API_REVISION: &str = #revision;
        },
        quote! {
            #[doc = "The human-readable API title."]
            pub const API_TITLE: &str = #title;
        },
        quote! {
            #[doc = "The root URL under which API services live."]
            pub const ROOT_URL: &str = #root_url;
        },
        quote! {
            #[doc = "The path used for REST requests."]
            pub const SERVICE_PATH: &str = #service_path;
        },
        quote! {
            #[doc = "The path used for REST batch requests."]
            pub const BATCH_PATH: &str = #batch_path;
        },
        quote! {
            #[doc = "The deprecated base URL for REST requests."]
            pub const BASE_URL: &str = #base_url;
        },
    ]);
    module
}

fn generate_protocol() -> GeneratedModule {
    let mut module = GeneratedModule::new("protocol");
    module.set_documentation(
        "Transport-independent protocol descriptors generated from the Discovery document.",
    );
    module.add_items([
        quote! {
            #[doc = "Static metadata for a method parameter."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct Parameter {
                #[doc = "The parameter name in the Discovery document."]
                pub name: &'static str,
                #[doc = "The REST location of the parameter."]
                pub location: &'static str,
                #[doc = "Whether the parameter must be supplied."]
                pub required: bool,
                #[doc = "Whether the parameter may occur more than once."]
                pub repeated: bool,
                #[doc = "The declared schema type of the parameter."]
                pub schema_type: &'static str,
            }
        },
        quote! {
            #[doc = "Static metadata for a REST method."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct Method {
                #[doc = "The method identifier."]
                pub id: &'static str,
                #[doc = "The HTTP method used to invoke the operation."]
                pub http_method: &'static str,
                #[doc = "The URI path of the method."]
                pub path: &'static str,
                #[doc = "The RFC 6570 URI path, when supplied."]
                pub flat_path: ::core::option::Option<&'static str>,
                #[doc = "The parameters accepted by the method."]
                pub parameters: &'static [Parameter],
                #[doc = "The request schema reference, when supplied."]
                pub request: ::core::option::Option<&'static str>,
                #[doc = "The response schema reference, when supplied."]
                pub response: ::core::option::Option<&'static str>,
                #[doc = "The OAuth scopes applicable to the method."]
                pub scopes: &'static [&'static str],
            }
        },
        quote! {
            #[doc = "Static metadata for a REST resource."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct Resource {
                #[doc = "The resource name."]
                pub name: &'static str,
                #[doc = "Methods exposed directly by this resource."]
                pub methods: &'static [Method],
                #[doc = "Resources nested below this resource."]
                pub resources: &'static [Resource],
            }
        },
        quote! {
            #[doc = "A generated request representation for a REST method."]
            pub trait MethodRequest {
                #[doc = "The static metadata for the represented method."]
                const DESCRIPTOR: Method;
            }
        },
    ]);
    module
}
