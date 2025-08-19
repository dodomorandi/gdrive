pub mod errors;

use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_recursion::async_recursion;

use super::{FileLike, FileTreeLike, FolderInfoLike, FolderLike};
use crate::common::{file_info::FileInfo, file_tree_like, id_gen::IdGen};

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
}

impl FileTreeLike for FileTree {
    type Folder = Folder;

    fn root(&self) -> &Self::Folder {
        &self.root
    }
}

type Node = file_tree_like::Node<Folder>;

#[derive(Debug, Clone)]
pub struct FolderInfo {
    pub name: String,
    pub path: PathBuf,
    pub parent: Option<Arc<FolderInfo>>,
    pub drive_id: String,
}

#[derive(Debug, Clone)]
pub struct Folder {
    pub info: Arc<FolderInfo>,
    pub children: Vec<Node>,
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
            info: Arc::new(FolderInfo {
                name,
                path: path.to_path_buf(),
                parent: parent.map(|folder| Arc::clone(&folder.info)),
                drive_id,
            }),
            children: Vec::new(),
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
                let node = Node::Folder(folder);
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
                let node = Node::File(file);
                children.push(node);
            } else {
                return Err(E::UnknownFileType(path));
            }
        }

        folder.children = children;

        Ok(folder)
    }

    #[must_use]
    pub fn relative_path(&self) -> &Path {
        get_relative_path(&self.info.path, &self.info)
    }
}

impl FolderLike for Folder {
    type File = File;
    type Info = FolderInfo;

    fn children(&self) -> &[file_tree_like::Node<Self>] {
        &self.children
    }

    fn info(&self) -> &Arc<Self::Info> {
        &self.info
    }
}

impl FolderInfo {
    fn root(&self) -> &Self {
        let mut folder = self;
        while let Some(parent) = &folder.parent {
            folder = parent;
        }
        folder
    }
}

impl FolderInfoLike for FolderInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: mime::Mime,
    pub parent: Arc<FolderInfo>,
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
            parent: Arc::clone(&parent.info),
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
            name: Cow::Borrowed(&self.name),
            size: self.size,
            mime_type: Cow::Borrowed(&self.mime_type),
            parents,
        }
    }
}

impl FileLike for File {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> u64 {
        self.size
    }
}

fn get_relative_path<'a>(path: &'a Path, folder_info: &FolderInfo) -> &'a Path {
    folder_info
        .root()
        .path
        .parent()
        .and_then(|root_path| path.strip_prefix(root_path).ok())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
    };

    use super::{File, Folder, Node};
    use crate::common::{drive_file::MIME_TYPE_CSV_MIME, file_tree::FolderInfo, FolderLike};

    #[test]
    fn folder_folders_recursive() {
        let folder_a = Arc::new(super::FolderInfo {
            name: "a".to_string(),
            path: PathBuf::from("a"),
            parent: None,
            drive_id: "a".to_string(),
        });
        let folder_b = Arc::new(FolderInfo {
            name: "b".to_string(),
            path: PathBuf::from("a/b"),
            parent: Some(Arc::clone(&folder_a)),
            drive_id: "b".to_string(),
        });
        let folder_c = Arc::new(FolderInfo {
            name: "c".to_string(),
            path: PathBuf::from("a/c"),
            parent: Some(Arc::clone(&folder_a)),
            drive_id: "c".to_string(),
        });
        let folder_e = Arc::new(FolderInfo {
            name: "e".to_string(),
            path: PathBuf::from("a/b/e"),
            parent: Some(Arc::clone(&folder_b)),
            drive_id: "e".to_string(),
        });

        let folder = Folder {
            info: Arc::clone(&folder_a),
            children: vec![
                Node::Folder(Folder {
                    info: folder_b,
                    children: vec![Node::Folder(Folder {
                        info: folder_e,
                        children: vec![],
                    })],
                }),
                Node::Folder(Folder {
                    info: Arc::clone(&folder_c),
                    children: vec![Node::File(File {
                        name: "f".to_string(),
                        path: PathBuf::from("a/c/f"),
                        size: 12,
                        mime_type: MIME_TYPE_CSV_MIME.clone(),
                        parent: Arc::clone(&folder_c),
                        drive_id: "f".to_string(),
                    })],
                }),
                Node::File(File {
                    name: "d".to_string(),
                    path: PathBuf::from("a/d"),
                    size: 12,
                    mime_type: MIME_TYPE_CSV_MIME.clone(),
                    parent: folder_a,
                    drive_id: "d".to_string(),
                }),
            ],
        };

        folder
            .folders_recursive()
            .iter()
            .map(|folder| folder.info.path.as_path())
            .eq([
                Path::new("a"),
                Path::new("a/b"),
                Path::new("a/b/e"),
                Path::new("a/c"),
                Path::new("a/c/f"),
                Path::new("a/d"),
            ]);
    }

    #[test]
    fn check_folder_not_leaking() {
        let folder_a = Arc::new(super::FolderInfo {
            name: "a".to_string(),
            path: PathBuf::from("a"),
            parent: None,
            drive_id: "a".to_string(),
        });
        let folder_b = Arc::new(FolderInfo {
            name: "b".to_string(),
            path: PathBuf::from("a/b"),
            parent: Some(Arc::clone(&folder_a)),
            drive_id: "b".to_string(),
        });

        let weak_folder_a = Arc::downgrade(&folder_a);
        let weak_folder_b = Arc::downgrade(&folder_b);

        let folder = Folder {
            info: folder_a,
            children: vec![Node::Folder(Folder {
                info: folder_b,
                children: vec![],
            })],
        };
        drop(folder);

        assert_eq!(weak_folder_a.strong_count(), 0);
        assert_eq!(weak_folder_b.strong_count(), 0);
    }
}
