use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EmptyFile;

impl Read for EmptyFile {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl Seek for EmptyFile {
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}
