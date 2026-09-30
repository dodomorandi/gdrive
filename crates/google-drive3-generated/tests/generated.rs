use borrowed_or_owned::{BorrowModel, ToOwnedModel};
use google_drive3_generated::{metadata, resources, schemas};

#[test]
fn exposes_drive_metadata_and_resources() {
    assert_eq!(metadata::API_ID, "drive:v3");
    assert_eq!(metadata::API_VERSION, "v3");
    assert!(resources::files::DESCRIPTOR
        .methods
        .iter()
        .any(|method| method.id == "drive.files.get"));
}

#[test]
fn exposes_generated_file_schema() {
    let _borrowed = resources::files::get::borrowed::Get::new("file-id");
    let _owned = resources::files::get::owned::Get::new("file-id".to_owned());
    let _cow = resources::files::get::cow::Get::new("file-id");

    assert_eq!(
        resources::files::get::borrowed::Get::DESCRIPTOR.http_method,
        "GET"
    );
    assert!(resources::files::get::borrowed::Get::DESCRIPTOR
        .path
        .contains("{fileId}"));
}

#[test]
fn method_response_alias_resolves_to_the_owned_schema() {
    let file: resources::files::get::Response = schemas::owned::File {
        id: Some("file-id".to_owned()),
        ..schemas::owned::File::default()
    };

    assert_eq!(file.id.as_deref(), Some("file-id"));
}

#[test]
fn owned_schema_round_trips_through_serde() {
    let file: schemas::owned::File =
        serde_json::from_str(r#"{"id":"file-id","name":"report.txt","size":"42"}"#).unwrap();
    assert_eq!(file.id.as_deref(), Some("file-id"));
    assert_eq!(file.name.as_deref(), Some("report.txt"));
    assert_eq!(file.size.as_deref(), Some("42"));

    let json = serde_json::to_value(&file).unwrap();
    assert_eq!(json["id"], "file-id");
    assert!(json.get("trashed").is_none());
}

#[test]
fn borrowed_schema_borrows_from_the_source_buffer() {
    let document = r#"{"id":"file-id","name":"report.txt"}"#;
    let parsed: serde_json::Value = serde_json::from_str(document).unwrap();
    let name = parsed["name"].as_str().unwrap();
    let file = schemas::borrowed::File {
        id: Some("file-id"),
        name: Some(name),
        ..schemas::borrowed::File::default()
    };

    assert_eq!(file.id, Some("file-id"));
    assert_eq!(file.name, Some("report.txt"));
    assert_eq!(file.name.unwrap().as_ptr(), name.as_ptr());
}

#[test]
fn smart_reference_holds_a_borrowed_view_or_an_owned_model() {
    let borrowed = schemas::borrowed::File {
        id: Some("file-id"),
        name: Some("report.txt"),
        ..schemas::borrowed::File::default()
    };
    let from_view = schemas::cow::File::borrowed(&borrowed);
    let from_owned = schemas::cow::File::owned(schemas::owned::File {
        id: Some("file-id".to_owned()),
        name: Some("report.txt".to_owned()),
        ..schemas::owned::File::default()
    });

    assert!(from_view.is_borrowed());
    assert!(from_owned.is_owned());
    assert_eq!(from_view.as_borrowed().unwrap().id, Some("file-id"));
    assert_eq!(
        from_view.as_borrowed().unwrap().id.unwrap().as_ptr(),
        borrowed.id.unwrap().as_ptr()
    );
    assert!(from_owned.as_borrowed().is_none());

    let converted = from_view.into_owned();
    assert_eq!(converted.id.as_deref(), Some("file-id"));
    assert_eq!(converted.name.as_deref(), Some("report.txt"));
    assert!(converted.id.as_deref().unwrap().as_ptr() != "file-id".as_ptr());
}

#[test]
fn to_owned_model_copies_every_borrowed_value() {
    let owners = [schemas::borrowed::User {
        display_name: Some("Ada"),
        ..schemas::borrowed::User::default()
    }];
    let app_properties = std::collections::BTreeMap::from([(String::from("team"), "core")]);
    let borrowed = schemas::borrowed::File {
        id: Some("file-id"),
        name: Some("report.txt"),
        parents: Some(std::borrow::Cow::Borrowed(&["root"][..])),
        owners: Some(std::borrow::Cow::Borrowed(&owners[..])),
        app_properties: Some(std::borrow::Cow::Borrowed(&app_properties)),
        ..schemas::borrowed::File::default()
    };

    let owned = borrowed.to_owned_model();

    assert_eq!(owned.id.as_deref(), Some("file-id"));
    assert_eq!(owned.name.as_deref(), Some("report.txt"));
    assert_eq!(owned.parents.as_deref(), Some(&[String::from("root")][..]));
    let owners = owned.owners.as_ref().unwrap();
    assert_eq!(owners[0].display_name.as_deref(), Some("Ada"));
    assert_eq!(owned.app_properties.as_ref().unwrap()["team"], "core");
    // Every borrowed value was copied, so nothing aliases the borrowed model.
    assert!(owned.id.as_deref().unwrap().as_ptr() != "file-id".as_ptr());
    assert!(owners[0].display_name.as_deref().unwrap().as_ptr() != "Ada".as_ptr());
}

#[test]
fn as_borrowed_lends_a_view_that_borrows_the_owned_model() {
    let owned = schemas::owned::File {
        id: Some("file-id".to_owned()),
        name: Some("report.txt".to_owned()),
        parents: Some(vec![String::from("root")]),
        owners: Some(vec![schemas::owned::User {
            display_name: Some("Ada".to_owned()),
            ..schemas::owned::User::default()
        }]),
        app_properties: Some(std::collections::BTreeMap::from([(
            String::from("team"),
            "core".to_owned(),
        )])),
        ..schemas::owned::File::default()
    };
    let id = owned.id.as_deref().unwrap().as_ptr();
    let parents = owned.parents.as_ref().unwrap()[0].as_ptr();
    let display_name = owned.owners.as_ref().unwrap()[0]
        .display_name
        .as_deref()
        .unwrap()
        .as_ptr();
    let app_property = owned.app_properties.as_ref().unwrap()["team"].as_ptr();

    let view = owned.as_borrowed();

    assert_eq!(view.id, Some("file-id"));
    assert_eq!(view.name, Some("report.txt"));
    // Scalars and map values are borrowed in place; only the containers are copied.
    assert!(view.id.unwrap().as_ptr() == id);
    assert_eq!(view.parents.as_deref().unwrap()[0], "root");
    assert!(view.parents.as_deref().unwrap()[0].as_ptr() == parents);
    assert_eq!(view.owners.as_deref().unwrap()[0].display_name, Some("Ada"));
    assert!(
        view.owners.as_deref().unwrap()[0]
            .display_name
            .unwrap()
            .as_ptr()
            == display_name
    );
    assert_eq!(view.app_properties.as_deref().unwrap()["team"], "core");
    assert!(view.app_properties.as_deref().unwrap()["team"].as_ptr() == app_property);
}
