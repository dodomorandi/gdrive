use std::{ops::Not, path::Path};

use google_api_codegen::{generate, GeneratedModule, GenerationError, IdentifierKind};
use google_api_discovery::RestDescription;
use quote::quote;

const DESCRIPTION: &str = r#"
{
  "kind": "discovery#restDescription",
  "discoveryVersion": "v1",
  "id": "example:v1",
  "name": "example",
  "version": "v1",
  "revision": "20260924",
  "title": "Example API",
  "description": "An example API",
  "icons": {"x16": "https://example.test/16.png", "x32": "https://example.test/32.png"},
  "documentationLink": "https://example.test/docs",
  "protocol": "rest",
  "baseUrl": "https://example.googleapis.com/example/v1/",
  "basePath": "/example/v1/",
  "rootUrl": "https://example.googleapis.com/",
  "servicePath": "example/v1/",
  "batchPath": "batch",
  "parameters": {
    "alt": {"type": "string", "location": "query", "description": "Response format."}
  },
  "schemas": {
    "Item": {
      "description": "An item in the collection.",
      "properties": {
        "id": {"type": "string", "description": "The item identifier."},
        "children": {"type": "array", "items": {"$ref": "Item"}, "description": "Nested items."},
        "status": {"type": "string", "enum": ["ready", "pending"], "enumDescriptions": ["Ready state", "Pending state"]}
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
    "files": {
      "methods": {
        "get": {
          "id": "files.get",
          "path": "files/{fileId}",
          "httpMethod": "GET",
          "description": "Get a file",
          "parameters": {
            "fileId": {"type": "string", "required": true, "location": "path", "description": "The file identifier."},
            "pageSize": {"type": "integer", "format": "int32", "location": "query", "description": "The maximum number of results."}
          },
          "request": {"$ref": "Item"},
          "response": {"$ref": "Item"},
          "scopes": ["https://example.test/auth/example"]
        }
      },
      "resources": {
        "comments": {
          "methods": {}
        }
      }
    }
  },
  "ownerDomain": "example.test",
  "ownerName": "Example"
}
"#;

fn description() -> RestDescription {
    serde_json::from_str(DESCRIPTION).unwrap()
}

#[test]
fn generates_file_and_module_trees() {
    let generated = generate(&description()).unwrap();
    let paths = generated
        .files
        .iter()
        .map(google_api_codegen::GeneratedFile::path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        [
            "metadata.rs",
            "protocol.rs",
            "schemas.rs",
            "api.rs",
            "resources.rs"
        ]
    );

    let resources = generated
        .files
        .iter()
        .find(|file| file.path() == Path::new("resources.rs"))
        .unwrap()
        .module();
    let resource_names = resources
        .children()
        .map(GeneratedModule::name)
        .collect::<Vec<_>>();
    assert_eq!(resource_names, ["files"]);
    let files = resources.children().next().unwrap();
    assert_eq!(
        files
            .children()
            .map(GeneratedModule::name)
            .collect::<Vec<_>>(),
        ["comments", "methods"]
    );

    let output = quote!(#generated).to_string();
    assert!(output.contains("pub mod schemas"));
    assert!(output.contains("pub struct Item"));
    assert!(output.contains("pub id"));
    assert!(output.contains("pub children"));
    assert!(output.contains("pub mod resources"));
    assert!(output.contains("pub mod files"));
    assert!(output.contains("pub mod comments"));
    assert!(output.contains("http_method"));
    assert!(output.contains("pub type Response"));
    assert!(output.contains("pub mod owned"));
    assert!(output.contains("pub mod cow"));
    assert!(output.contains("pub trait MethodRequest"));
    assert!(output.contains("An item in the collection."));
    assert!(output.contains("The item identifier."));
    assert!(output.contains("Ready state"));
    assert!(output.contains("Get a file"));
    assert!(output.contains("The file identifier."));
    assert!(output.contains("Typed request values for the `files` resource."));
}

#[test]
fn omits_empty_api_and_method_modules() {
    let mut document = description();
    document.methods = None;
    document.resources.clear();
    let generated = generate(&document).unwrap();
    let paths = generated
        .files
        .iter()
        .map(google_api_codegen::GeneratedFile::path)
        .collect::<Vec<_>>();

    assert_eq!(paths, ["metadata.rs", "protocol.rs", "schemas.rs"]);

    document.resources.insert(
        "empty".to_owned(),
        serde_json::from_str(r#"{"methods": {}}"#).unwrap(),
    );
    let generated = generate(&document).unwrap();
    let resources = generated
        .files
        .iter()
        .find(|file| file.path() == Path::new("resources.rs"))
        .unwrap()
        .module();
    let empty = resources
        .children()
        .find(|module| module.name() == "empty")
        .unwrap();

    assert!(empty.children().next().is_none());
}

#[test]
fn file_modules_and_module_nodes_render_through_to_tokens() {
    let generated = generate(&description()).unwrap();
    let schemas = generated
        .files
        .iter()
        .find(|file| file.path() == Path::new("schemas.rs"))
        .unwrap();
    let schemas_body = quote!(#schemas).to_string();
    assert!(schemas_body.contains("pub struct Item"));
    assert!(schemas_body.contains("pub mod schemas").not());

    let declarations = generated.file_declarations();
    let declarations = quote!(#declarations).to_string();
    assert!(declarations.contains("pub mod schemas"));

    let mut child = GeneratedModule::new("child");
    child.add_item(quote! { pub const VALUE: u8 = 1; });
    let mut root = GeneratedModule::new("root");
    root.add_module(child).unwrap();
    assert!(quote!(#root).to_string().contains("pub mod child"));
}

#[test]
fn unresolved_schema_references_are_reported() {
    let mut document = description();
    document
        .resources
        .get_mut("files")
        .unwrap()
        .methods
        .get_mut("get")
        .unwrap()
        .response = Some(google_api_discovery::models::SchemaRef {
        schema_ref: "Missing".to_owned(),
        parameter_name: None,
    });

    let error = generate(&document).unwrap_err();
    assert!(matches!(
        error,
        GenerationError::UnknownSchema { ref reference } if reference == "Missing"
    ));
}

#[test]
fn colliding_schema_names_are_reported() {
    let mut document = description();
    document.schemas.insert(
        "item-value".to_owned(),
        google_api_discovery::models::JsonSchema::default(),
    );
    document.schemas.insert(
        "item_value".to_owned(),
        google_api_discovery::models::JsonSchema::default(),
    );

    let error = generate(&document).unwrap_err();
    assert!(matches!(
        error,
        GenerationError::DuplicateIdentifier {
            kind: IdentifierKind::Schema,
            ..
        }
    ));
}
