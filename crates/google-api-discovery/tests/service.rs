use std::borrow::Cow;

use google_api_discovery::kind;
use google_api_discovery::{DirectoryList, DiscoveryService, ListApisParams, RestDescription};

const MINIMAL_DESCRIPTION: &[u8] = br#"{
    "kind": "discovery#restDescription",
    "discoveryVersion": "v1",
    "id": "example:v1",
    "name": "example",
    "version": "v1",
    "revision": "20260924",
    "title": "Example API",
    "description": "An example",
    "icons": {"x16": "https://example.test/16.png", "x32": "https://example.test/32.png"},
    "documentationLink": "https://example.test/docs",
    "protocol": "rest",
    "baseUrl": "https://example.googleapis.com/example/v1/",
    "basePath": "/example/v1/",
    "rootUrl": "https://example.googleapis.com/",
    "servicePath": "example/v1/",
    "batchPath": "batch",
    "parameters": {},
    "schemas": {},
    "resources": {},
    "ownerDomain": "example.test",
    "ownerName": "Example"
}"#;

const NESTED_DESCRIPTION: &[u8] = br#"{
    "kind": "discovery#restDescription",
    "discoveryVersion": "v1",
    "id": "example:v1",
    "name": "example",
    "version": "v1",
    "revision": "20260924",
    "title": "Example API",
    "description": "An example",
    "icons": {"x16": "https://example.test/16.png", "x32": "https://example.test/32.png"},
    "mtlsRootUrl": "https://example.mtls.googleapis.com/",
    "ownerDomain": "example.test",
    "ownerName": "Example",
    "documentationLink": "https://example.test/docs",
    "protocol": "rest",
    "rootUrl": "https://example.googleapis.com/",
    "servicePath": "example/v1/",
    "baseUrl": "https://example.googleapis.com/example/v1/",
    "basePath": "/example/v1/",
    "batchPath": "batch",
    "endpoints": [{
        "endpointUrl": "https://example.googleapis.com/",
        "location": "global",
        "deprecated": false,
        "description": "Global"
    }],
    "parameters": {
        "quotaUser": {
            "type": "string",
            "location": "query",
            "description": "Quota user"
        }
    },
    "auth": {
        "oauth2": {
            "scopes": {
                "https://example.test/auth/example": {
                    "description": "Example scope"
                }
            }
        }
    },
    "features": ["dataWrapper"],
    "schemas": {
        "Item": {
            "id": "Item",
            "type": "object",
            "description": "An item",
            "required": true,
            "properties": {
                "id": {"type": "string", "readOnly": true},
                "children": {
                    "type": "array",
                    "items": {"$ref": "Item"}
                }
            },
            "variant": {
                "discriminant": "kind",
                "map": [{"$ref": "Item", "type_value": "one"}]
            }
        }
    },
    "methods": {
        "root": {
            "id": "root",
            "path": "",
            "httpMethod": "GET",
            "description": "Root method",
            "parameters": {},
            "response": {"$ref": "Item"}
        }
    },
    "resources": {
        "items": {
            "deprecated": false,
            "methods": {
                "list": {
                    "id": "items.list",
                    "path": "items",
                    "flatPath": "items",
                    "httpMethod": "GET",
                    "description": "List items",
                    "parameters": {
                        "pageSize": {
                            "type": "integer",
                            "format": "int32",
                            "default": "10",
                            "minimum": "1",
                            "maximum": "1000",
                            "location": "query"
                        }
                    },
                    "parameterOrder": ["pageSize"],
                    "response": {"$ref": "Item"},
                    "supportsMediaDownload": true,
                    "supportsMediaUpload": true,
                    "mediaUpload": {
                        "accept": ["application/json"],
                        "maxSize": "1MB",
                        "protocols": {
                            "simple": {"multipart": true, "path": "upload"},
                            "resumable": {"multipart": false, "path": "resumable"}
                        }
                    }
                }
            },
            "resources": {
                "nested": {"methods": {}}
            }
        }
    }
}"#;

#[test]
fn list_url_uses_documented_endpoint() {
    let url = DiscoveryService::new().list_apis_url(&ListApisParams::new());

    assert_eq!(url, "https://discovery.googleapis.com/discovery/v1/apis");
}

#[test]
fn service_fields_are_public_configuration() {
    let mut service = DiscoveryService::default();
    service.base_url = Cow::Borrowed("https://example.test/discovery/v1");
    service.document_base_url = Cow::Borrowed("https://example.test/documents/v1");

    assert_eq!(
        service.list_apis_url(&ListApisParams::new()),
        "https://example.test/discovery/v1/apis"
    );

    service.document_base_url = Cow::Owned(String::from("https://other.test/documents/v1"));
    assert_eq!(
        service.get_rest_url("drive", "v3"),
        "https://other.test/documents/v1/apis/drive/v3/rest"
    );
}

#[test]
fn base_url_builder_only_changes_directory_url() {
    let service = DiscoveryService::new().with_base_url("https://example.test/discovery/v1");

    assert_eq!(
        service.list_apis_url(&ListApisParams::new()),
        "https://example.test/discovery/v1/apis"
    );
    assert_eq!(
        service.get_rest_url("drive", "v3"),
        "https://www.googleapis.com/discovery/v1/apis/drive/v3/rest"
    );

    let owned = DiscoveryService::new().with_base_url(String::from("https://owned.test"));
    assert_eq!(
        owned.list_apis_url(&ListApisParams::new()),
        "https://owned.test/apis"
    );
}

#[test]
fn list_parameters_omit_default_values() {
    let params = ListApisParams::new()
        .with_name("drive/v3 beta&x")
        .with_preferred(false);
    let url = DiscoveryService::new().list_apis_url(&params);

    assert_eq!(
        url,
        "https://discovery.googleapis.com/discovery/v1/apis?name=drive%2Fv3%20beta%26x"
    );
}

#[test]
fn empty_name_and_true_preferred_have_distinct_behavior() {
    let empty_name = DiscoveryService::new()
        .list_apis_url(&ListApisParams::new().with_name("").with_preferred(false));
    assert_eq!(
        empty_name,
        "https://discovery.googleapis.com/discovery/v1/apis"
    );

    let preferred = DiscoveryService::new().list_apis_url(
        &ListApisParams::new()
            .with_name("drive")
            .with_preferred(true),
    );
    assert_eq!(
        preferred,
        "https://discovery.googleapis.com/discovery/v1/apis?name=drive&preferred=true"
    );
}

#[test]
fn list_name_accepts_borrowed_and_owned_values() {
    let name = String::from("drive");
    let borrowed = ListApisParams::new().with_name(name.as_str());
    assert_eq!(
        DiscoveryService::new().list_apis_url(&borrowed),
        "https://discovery.googleapis.com/discovery/v1/apis?name=drive"
    );

    let owned = ListApisParams::new().with_name(String::from("drive"));
    assert_eq!(
        DiscoveryService::new().list_apis_url(&owned),
        "https://discovery.googleapis.com/discovery/v1/apis?name=drive"
    );
}

#[test]
fn percent_encoding_uses_uppercase_zero_padded_hex() {
    let params = ListApisParams::new().with_name("a/\0é");
    let url = DiscoveryService::new().list_apis_url(&params);

    assert_eq!(
        url,
        "https://discovery.googleapis.com/discovery/v1/apis?name=a%2F%00%C3%A9"
    );
}

#[test]
fn get_rest_url_uses_documented_path_and_encodes_segments() {
    let url = DiscoveryService::new().get_rest_url("drive/v1", "v3 beta");

    assert_eq!(
        url,
        "https://www.googleapis.com/discovery/v1/apis/drive%2Fv1/v3%20beta/rest"
    );
}

#[test]
fn response_bodies_parse_directly_with_serde() {
    let directory: DirectoryList = serde_json::from_slice(
        br#"{"kind":"discovery#directoryList","discoveryVersion":"v1","items":[]}"#,
    )
    .unwrap();
    assert!(matches!(directory.kind, kind::DirectoryList::DirectoryList));
    assert!(directory.items.is_empty());

    let document: RestDescription = serde_json::from_slice(MINIMAL_DESCRIPTION).unwrap();
    assert!(matches!(
        document.kind,
        kind::RestDescription::RestDescription
    ));
    assert_eq!(document.id, "example:v1");
    assert!(document.parameters.is_empty());
}

#[test]
fn kind_values_round_trip_and_reject_other_strings() {
    assert_eq!(
        serde_json::to_string(&kind::DirectoryList::DirectoryList).unwrap(),
        r#""discovery#directoryList""#
    );
    assert_eq!(
        serde_json::to_string(&kind::DirectoryItem::DirectoryItem).unwrap(),
        r#""discovery#directoryItem""#
    );
    assert_eq!(
        serde_json::to_string(&kind::RestDescription::RestDescription).unwrap(),
        r#""discovery#restDescription""#
    );
    assert!(serde_json::from_str::<kind::DirectoryList>(r#""discovery#directoryList""#).is_ok());
    assert!(serde_json::from_str::<kind::DirectoryItem>(r#""discovery#directoryItem""#).is_ok());
    assert!(
        serde_json::from_str::<kind::RestDescription>(r#""discovery#restDescription""#).is_ok()
    );

    assert!(serde_json::from_str::<kind::DirectoryList>(r#""discovery#restDescription""#).is_err());
    assert!(serde_json::from_str::<kind::DirectoryItem>(r#""discovery#directoryList""#).is_err());
    assert!(serde_json::from_str::<kind::RestDescription>(r#""other""#).is_err());
    assert!(
        serde_json::from_slice::<DirectoryList>(br#"{"kind":"discovery#restDescription"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_slice::<RestDescription>(br#"{"kind":"discovery#directoryList"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_slice::<RestDescription>(br#"{"kind":"discovery#restDescription"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_slice::<DirectoryList>(br#"{"discoveryVersion":"v1","items":[]}"#)
            .is_err()
    );
    assert!(serde_json::from_slice::<DirectoryList>(
        br#"{"kind":"discovery#directoryList","discoveryVersion":"v1"}"#
    )
    .is_err());
}

#[test]
fn serializing_omits_absent_optional_fields() {
    let document: RestDescription = serde_json::from_slice(MINIMAL_DESCRIPTION).unwrap();
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["kind"], "discovery#restDescription");
    assert!(value.get("canonicalName").is_none());
}

#[test]
fn directory_response_decodes_documented_fields() {
    let directory: DirectoryList = serde_json::from_slice(
        br#"{
            "kind": "discovery#directoryList",
            "discoveryVersion": "v1",
            "items": [{
                "kind": "discovery#directoryItem",
                "id": "drive:v3",
                "name": "drive",
                "version": "v3",
                "title": "Google Drive API",
                "description": "Manage files",
                "discoveryRestUrl": "https://www.googleapis.com/discovery/v1/apis/drive/v3/rest",
                "icons": {"x16": "https://example.test/16.png", "x32": "https://example.test/32.png"},
                "documentationLink": "https://developers.google.com/drive/",
                "preferred": true
            }]
        }"#,
    )
    .unwrap();

    assert!(matches!(directory.kind, kind::DirectoryList::DirectoryList));
    assert_eq!(directory.discovery_version, "v1");
    assert_eq!(directory.items.len(), 1);
    let item = &directory.items[0];
    assert!(matches!(item.kind, kind::DirectoryItem::DirectoryItem));
    assert_eq!(item.id, "drive:v3");
    assert_eq!(item.name, "drive");
    assert_eq!(item.version, "v3");
    assert!(item.preferred);
    assert!(item.discovery_link.is_none());
    assert!(item.labels.is_none());
    assert_eq!(item.icons.x16, "https://example.test/16.png");
}

#[test]
fn rest_description_decodes_nested_resources_schemas_and_media() {
    let document: RestDescription = serde_json::from_slice(NESTED_DESCRIPTION).unwrap();

    assert!(matches!(
        document.kind,
        kind::RestDescription::RestDescription
    ));
    assert_eq!(document.id, "example:v1");
    assert_eq!(
        document.parameters["quotaUser"].location.as_deref(),
        Some("query")
    );
    assert_eq!(document.auth.as_ref().unwrap().oauth2.scopes.len(), 1);

    let item_schema = &document.schemas["Item"];
    assert!(item_schema.required.unwrap());
    assert_eq!(item_schema.variant.as_ref().unwrap().discriminant, "kind");
    assert_eq!(
        item_schema.variant.as_ref().unwrap().map[0].type_value,
        "one"
    );
    let properties = item_schema.properties.as_ref().unwrap();
    assert!(properties["id"].read_only.unwrap());
    assert_eq!(
        properties["children"]
            .items
            .as_ref()
            .unwrap()
            .schema_ref
            .as_deref(),
        Some("Item")
    );

    let items_resource = &document.resources["items"];
    let list_method = &items_resource.methods["list"];
    assert_eq!(list_method.http_method, "GET");
    assert_eq!(
        list_method.parameters["pageSize"].default.as_deref(),
        Some("10")
    );
    assert!(list_method.supports_media_download.unwrap());
    assert!(list_method
        .media_upload
        .as_ref()
        .unwrap()
        .protocols
        .resumable
        .is_some());
    assert!(items_resource
        .resources
        .as_ref()
        .unwrap()
        .contains_key("nested"));
}

#[test]
fn malformed_json_returns_serde_error() {
    let result = serde_json::from_slice::<RestDescription>(b"not json");

    assert!(result.is_err());
}
