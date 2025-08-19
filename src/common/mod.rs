pub mod account_archive;
pub mod delegate;
pub mod drive_file;
pub mod empty_file;
pub mod file_helper;
pub mod file_info;
pub mod file_tree;
pub mod file_tree_drive;
mod file_tree_like;
pub mod hub_helper;
pub mod id_gen;
pub mod md5_writer;
pub mod permission;
pub mod table;

pub(crate) use file_tree_like::{FileLike, FileTreeLike, FolderInfoLike, FolderLike};

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

#[cfg(test)]
mod test {
    use super::parse_md5_digest;

    #[test]
    fn parse_md5_digest_valid() {
        assert_eq!(
            parse_md5_digest("123456789abcdef01fedcba098765432").unwrap(),
            md5::Digest([
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x1f, 0xed, 0xcb, 0xa0, 0x98, 0x76,
                0x54, 0x32,
            ])
        );
    }

    #[test]
    fn parse_md5_digest_invalid() {
        assert!(parse_md5_digest("123456789abcdef01f3dcba09876542").is_none());
        assert!(parse_md5_digest("123456789abcdef01f3dcba09876543").is_none());
        assert!(parse_md5_digest("123456789abcdef01f3dcba0987654321").is_none());
        assert!(parse_md5_digest("123456789abcdef01f3dcba09876543210").is_none());
        assert!(parse_md5_digest("g23456789abcdef01f3dcba098765432").is_none());
    }
}
