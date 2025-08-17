pub mod errors;

use crate::common::file_info::FileInfo;
use crate::common::id_gen::IdGen;
use async_recursion::async_recursion;
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use super::FolderLike;

#[derive(Debug, Clone)]
pub struct FileTree {
    pub root: Folder,
}

impl FileTree {
    pub async fn from_path(path: &Path, ids: &mut IdGen<'_>) -> Result<FileTree, errors::FileTree> {
        let canonical_path = path
            .canonicalize()
            .map_err(errors::FileTree::Canonicalize)?;

        let root = Folder::from_path(&canonical_path, None, ids)
            .await
            .map_err(errors::FileTree::Folder)?;
        Ok(FileTree { root })
    }

    #[must_use]
    pub fn folders(&self) -> Vec<Folder> {
        let mut folders = vec![];

        folders.push(self.root.clone());
        let child_folders = self.root.folders_recursive();
        folders.extend(child_folders);

        folders.sort_by(|a, b| {
            let parent_count_a = a.ancestor_count();
            let parent_count_b = b.ancestor_count();

            parent_count_a
                .cmp(&parent_count_b)
                .then_with(|| a.name.cmp(&b.name))
        });

        folders
    }

    #[must_use]
    pub fn info(&self) -> TreeInfo {
        let mut file_count = 0;
        let mut folder_count = 0;
        let mut total_file_size = 0;

        for folder in self.folders() {
            folder_count += 1;

            for file in folder.files() {
                file_count += 1;
                total_file_size += file.size;
            }
        }

        TreeInfo {
            file_count,
            folder_count,
            total_file_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TreeInfo {
    pub file_count: u64,
    pub folder_count: u64,
    pub total_file_size: u64,
}

#[derive(Debug, Clone)]
pub enum Node {
    FolderNode(Folder),
    FileNode(File),
}

#[derive(Debug, Clone)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub parent: Option<Box<Folder>>,
    pub children: Vec<Node>,
    pub drive_id: String,
}

impl Folder {
    #[async_recursion]
    pub async fn from_path(
        path: &Path,
        parent: Option<&'async_recursion Folder>,
        ids: &mut IdGen<'_>,
    ) -> Result<Folder, errors::Folder> {
        use errors::Folder as E;

        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or(E::InvalidPath)?;

        let drive_id = ids.next().await.map_err(E::GenerateId)?;

        let mut folder = Folder {
            name,
            path: path.to_path_buf(),
            parent: parent.map(|folder| Box::new(folder.clone())),
            children: Vec::new(),
            drive_id,
        };

        let entries = fs::read_dir(path).map_err(E::ReadDir)?;
        let mut children = Vec::new();

        for e in entries {
            let entry = e.map_err(E::ReadDirEntry)?;
            let path = entry.path();

            if path.is_dir() {
                let folder = match Folder::from_path(&path, Some(&folder), ids).await {
                    Ok(folder) => folder,
                    Err(source) => {
                        return Err(E::Nested {
                            path,
                            source: Box::new(source),
                        });
                    }
                };
                let node = Node::FolderNode(folder);
                children.push(node);
            } else if path.is_symlink() {
                return Err(E::IsSymlink(path));
            } else if path.is_file() {
                let file = match File::from_path(&path, &folder, ids).await {
                    Ok(file) => file,
                    Err(source) => {
                        return Err(E::File { path, source });
                    }
                };
                let node = Node::FileNode(file);
                children.push(node);
            } else {
                return Err(E::UnknownFileType(path));
            }
        }

        folder.children = children;

        Ok(folder)
    }

    #[must_use]
    pub fn files(&self) -> Vec<File> {
        let mut files = vec![];

        for child in &self.children {
            if let Node::FileNode(file) = child {
                files.push(file.clone());
            }
        }

        files.sort_by(|a, b| a.name.cmp(&b.name));

        files
    }

    #[must_use]
    pub fn relative_path(&self) -> &Path {
        get_relative_path(&self.path, self)
    }

    #[must_use]
    pub fn folders_recursive(&self) -> Vec<Folder> {
        Folder::collect_folders_recursive(self)
    }

    #[must_use]
    pub fn ancestor_count(&self) -> usize {
        FolderLike::ancestor_count(self)
    }

    fn collect_folders_recursive(folder: &Folder) -> Vec<Folder> {
        let mut folders = vec![];

        folder.children.iter().for_each(|child| {
            if let Node::FolderNode(folder) = child {
                folders.push(folder.clone());
                let child_folders = Folder::collect_folders_recursive(folder);
                folders.extend(child_folders);
            }
        });

        folders
    }

    fn root(&self) -> &Self {
        let mut folder = self;
        while let Some(parent) = &folder.parent {
            folder = parent;
        }
        folder
    }
}

impl FolderLike for Folder {
    fn parent(&self) -> Option<&Self> {
        self.parent.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: mime::Mime,
    pub parent: Folder,
    pub drive_id: String,
}

impl File {
    pub async fn from_path(
        path: &Path,
        parent: &Folder,
        ids: &mut IdGen<'_>,
    ) -> Result<File, errors::File> {
        use errors::File as E;

        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or(E::InvalidPath)?;

        let os_file = fs::File::open(path).map_err(E::OpenFile)?;
        let size = os_file.metadata().map(|m| m.len()).unwrap_or(0);
        let mime_type = mime_guess::from_path(path)
            .first()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);
        let drive_id = ids.next().await.map_err(E::GenerateId)?;

        let file = File {
            name,
            path: path.to_path_buf(),
            size,
            mime_type,
            parent: parent.clone(),
            drive_id,
        };

        Ok(file)
    }

    #[must_use]
    pub fn relative_path(&self) -> &Path {
        get_relative_path(&self.path, &self.parent)
    }

    #[must_use]
    pub fn info(&self, parents: Option<Vec<String>>) -> FileInfo<'_> {
        FileInfo {
            name: self.name.clone(),
            size: self.size,
            mime_type: Cow::Borrowed(&self.mime_type),
            parents,
        }
    }
}

fn get_relative_path<'a>(path: &'a Path, folder: &Folder) -> &'a Path {
    folder
        .root()
        .path
        .parent()
        .and_then(|root_path| path.strip_prefix(root_path).ok())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::common::drive_file::MIME_TYPE_CSV_MIME;

    use super::{File, Folder, Node};

    #[test]
    fn folder_folders_recursive() {
        let folder = Folder {
            name: "a".to_string(),
            path: PathBuf::from("a"),
            parent: None,
            children: vec![
                Node::FolderNode(Folder {
                    name: "b".to_string(),
                    path: PathBuf::from("a/b"),
                    parent: None,
                    children: vec![Node::FolderNode(Folder {
                        name: "e".to_string(),
                        path: PathBuf::from("a/b/e"),
                        parent: None,
                        children: vec![],
                        drive_id: "e".to_string(),
                    })],
                    drive_id: "b".to_string(),
                }),
                Node::FolderNode(Folder {
                    name: "c".to_string(),
                    path: PathBuf::from("a/c"),
                    parent: None,
                    children: vec![Node::FileNode(File {
                        name: "f".to_string(),
                        path: PathBuf::from("a/c/f"),
                        size: 12,
                        mime_type: MIME_TYPE_CSV_MIME.clone(),
                        parent: Folder {
                            name: "c".to_string(),
                            path: PathBuf::from("a/c"),
                            parent: None,
                            children: vec![],
                            drive_id: "c".to_string(),
                        },
                        drive_id: "f".to_string(),
                    })],
                    drive_id: "c".to_string(),
                }),
                Node::FileNode(File {
                    name: "d".to_string(),
                    path: PathBuf::from("a/d"),
                    size: 12,
                    mime_type: MIME_TYPE_CSV_MIME.clone(),
                    parent: Folder {
                        name: "a".to_string(),
                        path: PathBuf::from("a"),
                        parent: None,
                        children: vec![],
                        drive_id: "a".to_string(),
                    },
                    drive_id: "d".to_string(),
                }),
            ],
            drive_id: "a".to_string(),
        };

        folder
            .folders_recursive()
            .iter()
            .map(|folder| folder.path.as_path())
            .eq([
                Path::new("a"),
                Path::new("a/b"),
                Path::new("a/b/e"),
                Path::new("a/c"),
                Path::new("a/c/f"),
                Path::new("a/d"),
            ]);
    }
}
