//! Generates parser-first, sans-I/O Rust modules from a Google Discovery document.
//!
//! The generator consumes the parser models from `google-api-discovery` and emits a hierarchical
//! [`Generated`] value. Its output types implement [`ToTokens`], so callers can render a complete
//! crate with `quote!(#generated)` or write each [`GeneratedFile`] separately. Generated crates
//! need `serde` with its `derive` feature, `serde_json`, and `borrowed-or-owned` as dependencies.
//! The generator never opens sockets, chooses an async runtime, or depends on an HTTP client.
//!
//! # Generated layout
//!
//! A generated crate contains one module per top-level artifact. Every method becomes one module
//! that holds its type aliases and one nested module per storage variant, so a method's types are
//! never spread across the resource.
//!
//! - `metadata` exposes the document's identity, revision, and URLs as constants.
//! - `protocol` exposes the `Resource`, `Method`, `Parameter`, and `MethodRequest` types that
//!   describe requests without performing them.
//! - `schemas::owned` holds `serde` models that own their data, and `schemas::borrowed` holds the
//!   same models borrowing from a caller-provided `'a` lifetime. Only the owned variant
//!   implements `Serialize` and `Deserialize` from `serde`.
//! - The two halves are related by the model pair from `borrowed-or-owned`: every borrowed model
//!   implements `ToOwnedModel`, which copies it into the owned model, and every owned model
//!   implements `BorrowModel`, which builds a borrowed view that borrows its fields in place.
//!   A borrowed model holds its nested models and collections in a `Cow`, so it can store the
//!   values that view produces.
//! - `schemas::cow` holds one alias per object schema, a `borrowed_or_owned::MaybeOwned` that is
//!   either a borrowed model or an owned one. `Cow` covers the cases where one type is enough,
//!   such as strings in request parameters and `Cow<'a, [schemas::cow::Item<'a>]>` collections.
//! - Enums carry no data, so a single definition in `schemas::owned` is shared by every variant
//!   and re-exported from the others.
//! - `api` and `resources::<name>` expose `DESCRIPTOR` constants for the API and each resource,
//!   and every method lives in `api::<method>` or `resources::<name>::<method>`. Such a module
//!   aliases the method's `RequestBody` and `Response` types and holds `borrowed`, `owned`, and
//!   `cow` submodules with the request struct for that storage.
//!
//! Types whose fields only contain `bool`, integer, number, and enum schemas are generated without
//! a lifetime parameter, because they carry no borrowed data.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod attributes;
mod error;
mod identifier;
mod output;
mod resource;
mod schema;

use std::{borrow::Cow, ops::Not};

use google_api_discovery::RestDescription;
use quote::quote;

pub use error::{GenerationError, IdentifierKind};
pub use output::{FileDeclarations, Generated, GeneratedFile, GeneratedModule};
pub use proc_macro2::TokenStream;
pub use quote::ToTokens;

use crate::{resource::generate_resources, schema::SchemaGenerator};

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
pub fn generate(description: &RestDescription) -> Result<Generated<'_>, GenerationError> {
    let mut schemas = SchemaGenerator::new(description)?;
    let named_schema_items = schemas.prepare_all()?;
    let (api, resources) = generate_resources(description, &mut schemas)?;
    let schemas = schemas.finish(&named_schema_items);

    let mut files = vec![
        GeneratedFile::new("metadata.rs", generate_metadata(description)),
        GeneratedFile::new("protocol.rs", generate_protocol()),
    ];
    if schemas.is_empty().not() {
        files.push(GeneratedFile::new("schemas.rs", schemas));
    }
    if api.is_empty().not() {
        files.push(GeneratedFile::new("api.rs", api));
    }
    if resources.is_empty().not() {
        files.push(GeneratedFile::new("resources.rs", resources));
    }
    Ok(Generated { files })
}

fn generate_metadata(description: &RestDescription) -> GeneratedModule<'_> {
    let mut module = GeneratedModule::new("metadata");
    let documentation = if description.description.trim().is_empty() {
        Cow::Owned(format!("Metadata for the `{}` API.", description.id))
    } else {
        Cow::Borrowed(&*description.description)
    };
    module.set_documentation(documentation);
    let RestDescription {
        id,
        name,
        version,
        revision,
        title,
        base_url,
        root_url,
        service_path,
        batch_path,
        ..
    } = &description;

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

fn generate_protocol() -> GeneratedModule<'static> {
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
