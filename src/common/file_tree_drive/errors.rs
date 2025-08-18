use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug)]
pub struct FileTreeDrive(pub super::Error);

impl Display for FileTreeDrive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unable to create a folder from file")
    }
}

impl Error for FileTreeDrive {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
