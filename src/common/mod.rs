pub mod account_archive;
pub mod delegate;
pub mod drive_file;
pub mod empty_file;
pub mod file_helper;
pub mod file_info;
pub mod file_tree;
pub mod file_tree_drive;
mod folder_like;
pub mod hub_helper;
pub mod id_gen;
pub mod md5_writer;
pub mod permission;
pub mod table;

use folder_like::FolderLike;

#[expect(unused)]
pub(crate) fn parse_md5_digest(s: &str) -> Option<md5::Digest> {
    const MD5_LEN: usize = 16;

    if s.len() != MD5_LEN * 2 {
        return None;
    }

    let (chunks, _) = s.as_bytes().as_chunks::<2>();
    let mut md5_bytes = [0; MD5_LEN];
    chunks
        .iter()
        .map(|bytes| {
            let s = std::str::from_utf8(bytes).ok()?;
            u8::from_str_radix(s, 16).ok()
        })
        .zip(&mut md5_bytes)
        .try_for_each(|(byte, out)| {
            *out = byte?;
            Some(())
        })?;

    Some(md5::Digest(md5_bytes))
}
