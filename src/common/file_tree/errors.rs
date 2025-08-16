use std::{
    error::Error,
    fmt::{self, Display},
    io,
};

#[derive(Debug)]
pub enum FileTree {
    Canonicalize(io::Error),
    Folder(super::Error),
}

impl Display for FileTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FileTree::Canonicalize(_) => "unable to canonicalize path",
            FileTree::Folder(_) => "unable to create folder tree from canonicalized path",
        };

        f.write_str(s)
    }
}

impl Error for FileTree {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileTree::Canonicalize(source) => Some(source),
            FileTree::Folder(source) => Some(source),
        }
    }
}
