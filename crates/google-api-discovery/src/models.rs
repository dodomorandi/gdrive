//! Serde models for the Google API Discovery Service resources.

use std::collections::BTreeMap;

use crate::kind;
use serde::{Deserialize, Serialize};

/// A list of APIs and API versions supported by the Discovery Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryList {
    /// Identifies the response as a Discovery Service directory list.
    pub kind: kind::DirectoryList,

    /// Identifies the Discovery API version used to generate the response.
    pub discovery_version: String,

    /// The API and version entries in the directory.
    pub items: Vec<DirectoryItem>,
}

/// One API and version entry in the APIs directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryItem {
    /// Identifies the entry as a Discovery Service directory item.
    pub kind: kind::DirectoryItem,

    /// The API identifier, such as `drive:v3`.
    pub id: String,

    /// The API name.
    pub name: String,

    /// The API version.
    pub version: String,

    /// The human-readable API title.
    pub title: String,

    /// A short description of the API.
    pub description: String,

    /// The URL of the API's REST discovery document.
    pub discovery_rest_url: String,

    /// A link to the discovery document, when supplied by the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_link: Option<String>,

    /// Links to the API's icons.
    pub icons: Icons,

    /// A link to human-readable API documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_link: Option<String>,

    /// Status labels for the API version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,

    /// Whether this is the preferred version of the API.
    pub preferred: bool,
}

/// Icon URLs supplied for an API or directory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Icons {
    /// The URL of the 16 by 16 icon.
    pub x16: String,

    /// The URL of the 32 by 32 icon.
    pub x32: String,
}

/// A machine-readable description of one API version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestDescription {
    /// Identifies the document as a REST discovery description.
    pub kind: kind::RestDescription,

    /// Identifies the Discovery API version used to generate the document.
    pub discovery_version: String,

    /// The document identifier, such as `drive:v3`.
    pub id: String,

    /// The API name.
    pub name: String,

    /// The API's canonical display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,

    /// The API version.
    pub version: String,

    /// The document revision.
    pub revision: String,

    /// The human-readable API title.
    pub title: String,

    /// A description of the API.
    pub description: String,

    /// Links to the API's icons.
    pub icons: Icons,

    /// A link to human-readable API documentation.
    pub documentation_link: String,

    /// Status labels for the API version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,

    /// The protocol described by the document.
    pub protocol: String,

    /// The deprecated base URL for REST requests.
    pub base_url: String,

    /// The deprecated base path for REST requests.
    pub base_path: String,

    /// The root URL under which API services live.
    pub root_url: String,

    /// The base path for REST requests.
    pub service_path: String,

    /// The path for REST batch requests.
    pub batch_path: String,

    /// Whether exponential backoff is enabled by default for suitable methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exponential_backoff_default: Option<bool>,

    /// Location-specific API endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<Endpoint>>,

    /// Parameters common to all methods in the API.
    pub parameters: BTreeMap<String, JsonSchema>,

    /// Authentication information for the API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    /// Supported API features.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,

    /// Schemas used by the API's methods.
    pub schemas: BTreeMap<String, JsonSchema>,

    /// API-level methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methods: Option<BTreeMap<String, RestMethod>>,

    /// Resources exposed by the API.
    pub resources: BTreeMap<String, RestResource>,

    /// The mTLS root URL, when supplied by the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtls_root_url: Option<String>,

    /// The domain that owns the API, when supplied by the document.
    pub owner_domain: String,

    /// The display name of the API owner, when supplied by the document.
    pub owner_name: String,

    /// Whether reserved expansion characters are fully encoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fully_encode_reserved_expansion: Option<bool>,

    /// Whether the document contains a version module marker.
    #[serde(
        default,
        rename = "version_module",
        skip_serializing_if = "Option::is_none"
    )]
    pub version_module: Option<bool>,

    /// The package path, when supplied by the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_path: Option<String>,

    /// The document `ETag`, when supplied by the service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

/// A location-specific endpoint for an API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    /// The endpoint target URL.
    pub endpoint_url: String,

    /// The endpoint location.
    pub location: String,

    /// Whether the endpoint is deprecated.
    pub deprecated: bool,

    /// A description of the endpoint.
    pub description: String,
}

/// Authentication information for an API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    /// OAuth 2.0 authentication information.
    pub oauth2: OAuth2,
}

/// OAuth 2.0 authentication information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2 {
    /// OAuth 2.0 scopes keyed by scope URL.
    pub scopes: BTreeMap<String, ScopeDescription>,
}

/// A description of one OAuth 2.0 scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeDescription {
    /// Human-readable scope description.
    pub description: String,
}

/// A resource containing methods and nested resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestResource {
    /// Methods exposed directly by the resource.
    pub methods: BTreeMap<String, RestMethod>,

    /// Whether the resource is deprecated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    /// Resources nested below this resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<BTreeMap<String, RestResource>>,
}

/// A REST method described by a discovery document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestMethod {
    /// The method's stable identifier.
    pub id: String,

    /// The API version accepted through an API version header or query parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// The URI path of the method.
    pub path: String,

    /// The URI path in RFC 6570 format, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flat_path: Option<String>,

    /// The HTTP method used to invoke the operation.
    pub http_method: String,

    /// A description of the method.
    pub description: String,

    /// Whether the method is deprecated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    /// Whether the method requires an `ETag` in a conditional request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag_required: Option<bool>,

    /// Parameters accepted by the method.
    pub parameters: BTreeMap<String, JsonSchema>,

    /// The order of required parameters in generated client signatures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_order: Option<Vec<String>>,

    /// The request schema reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<SchemaRef>,

    /// The response schema reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<SchemaRef>,

    /// OAuth 2.0 scopes applicable to the method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,

    /// Whether the method supports media downloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_media_download: Option<bool>,

    /// Whether the method supports media uploads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_media_upload: Option<bool>,

    /// Media upload details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_upload: Option<MediaUpload>,

    /// Whether the method supports subscriptions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_subscription: Option<bool>,

    /// Whether the media download service should be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_media_download_service: Option<bool>,

    /// The streaming type, when supplied by the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming_type: Option<String>,
}

/// A reference to a schema or request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRef {
    /// The referenced schema identifier.
    #[serde(rename = "$ref")]
    pub schema_ref: String,

    /// A deprecated request parameter name retained by some documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_name: Option<String>,
}

/// Media upload details for a method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaUpload {
    /// MIME media ranges accepted by the method.
    pub accept: Vec<String>,

    /// The maximum media size, such as `1MB` or `2GB`.
    pub max_size: String,

    /// Supported upload protocols.
    pub protocols: MediaUploadProtocols,
}

/// Upload protocols supported by a method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaUploadProtocols {
    /// Single-request upload support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simple: Option<MediaUploadProtocol>,

    /// Resumable upload support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumable: Option<MediaUploadProtocol>,
}

/// Details for one media upload protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaUploadProtocol {
    /// Whether multipart uploads are supported.
    pub multipart: bool,

    /// The URI path used for the upload.
    pub path: String,
}

/// A JSON Schema-like description used for schemas and method parameters.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchema {
    /// The schema identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The JSON type, such as `object` or `string`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,

    /// A reference to another schema.
    #[serde(default, rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,

    /// A description of the schema or parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The default value, represented as a string by the Discovery format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Whether the parameter is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Whether the schema is deprecated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    /// An additional format or validation hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// A regular expression constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// The minimum value, represented as a string by the Discovery format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<String>,

    /// The maximum value, represented as a string by the Discovery format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<String>,

    /// Allowed values.
    #[serde(default, rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,

    /// Descriptions corresponding to `enum_values`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_descriptions: Option<Vec<String>>,

    /// Deprecation status corresponding to `enum_values`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_deprecated: Option<Vec<bool>>,

    /// Whether the parameter may occur more than once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeated: Option<bool>,

    /// The REST location of a parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Properties of an object schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, JsonSchema>>,

    /// The schema for dynamic object properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<JsonSchema>>,

    /// The schema for array elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,

    /// Additional method-specific requirements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Whether the property is read-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    /// Variant metadata for discriminated schema types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<JsonSchemaVariant>,
}

/// Metadata describing a discriminated schema variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSchemaVariant {
    /// The name of the discriminant property.
    pub discriminant: String,

    /// The mapping from discriminant values to schema references.
    pub map: Vec<JsonSchemaVariantMap>,
}

/// One discriminant-value mapping in a schema variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSchemaVariantMap {
    /// The schema reference selected by the discriminant value.
    #[serde(rename = "$ref")]
    pub schema_ref: String,

    /// The discriminant value.
    pub type_value: String,
}

/// Additional information attached to a schema or parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotations {
    /// Methods that require the property.
    pub required: Vec<String>,
}

/// A method parameter description.
///
/// Discovery documents use the same schema shape for parameters and schemas.
pub type MethodParameter = JsonSchema;

/// A parameter description.
///
/// This alias is provided for callers that use the terminology from the
/// Discovery resource representation.
pub type Parameter = JsonSchema;
