pub mod errors;

use std::fs::File;
use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;

/// Creates an archive of the given source directory.
///
/// # Panics
///
/// The function panics if `src_path` terminates with a `..`.
pub fn create(src_path: &Path, archive_path: &Path) -> Result<(), errors::Create> {
    if src_path.exists().not() {
        return Err(errors::Create::SrcPathDoesNotExist);
    }

    if src_path.is_dir().not() {
        return Err(errors::Create::SrcPathNotDirectory);
    }

    let archive_file = File::create_new(archive_path).map_err(errors::Create::CreateArchive)?;
    let mut builder = tar::Builder::new(archive_file);

    let src_dir_name = src_path
        .file_name()
        .expect("`src_path` should not terminate with `..`");

    if let Err(source) = builder.append_dir_all(src_dir_name, src_path) {
        return Err(errors::Create::AppendDir {
            dir_path: PathBuf::from(src_dir_name),
            source,
        });
    }

    builder.finish().map_err(errors::Create::FinishArchive)?;

    Ok(())
}

pub fn unpack(archive_path: &Path, dst_path: &Path) -> Result<(), errors::Unpack> {
    if archive_path.exists().not() {
        return Err(errors::Unpack::ArchivePathDoesNotExist);
    }

    if dst_path.exists().not() {
        return Err(errors::Unpack::DstDoesNotExist);
    }

    let archive_file = File::open(archive_path).map_err(errors::Unpack::Open)?;
    let mut archive = tar::Archive::new(archive_file);
    archive.unpack(dst_path).map_err(errors::Unpack::Unpack)
}

pub fn get_account_name(archive_path: &Path) -> Result<String, errors::GetAccountName> {
    let archive_file = File::open(archive_path).map_err(errors::GetAccountName::Open)?;
    let mut archive = tar::Archive::new(archive_file);
    let entries = archive
        .entries()
        .map_err(errors::GetAccountName::ReadEntries)?;

    let mut dir_names = entries.filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path().ok()?;

        if entry.header().entry_type() == tar::EntryType::Directory {
            let file_name = path.file_name()?;
            Some(file_name.to_string_lossy().to_string())
        } else {
            None
        }
    });

    let name = dir_names
        .next()
        .ok_or(errors::GetAccountName::NoDirectories)?;
    if dir_names.next().is_some() {
        return Err(errors::GetAccountName::MultipleDirectories);
    }

    Ok(name)
}
