use std::{env, error::Error, fs, path::PathBuf};

use google_api_codegen::{generate, ToTokens};
use google_api_discovery::RestDescription;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let discovery_path = manifest_dir.join("discovery/drive-v3.json");
    println!("cargo:rerun-if-changed={}", discovery_path.display());

    let description: RestDescription = serde_json::from_slice(&fs::read(&discovery_path)?)?;
    let generated = generate(&description)?;
    let syntax: syn::File = syn::parse2(generated.to_token_stream())?;
    let source = prettyplease::unparse(&syntax);

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("drive_v3.rs"), source)?;
    Ok(())
}
