use google_drive3_generated::{metadata, resources};

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
    let _borrowed = resources::files::methods::Get::new("file-id");
    let _owned = resources::files::methods::owned::Get::new("file-id".to_owned());
    let _cow = resources::files::methods::cow::Get::new("file-id");

    assert_eq!(
        resources::files::methods::Get::DESCRIPTOR.http_method,
        "GET"
    );
    assert!(resources::files::methods::Get::DESCRIPTOR
        .path
        .contains("{fileId}"));
}
