pub mod errors;

use std::{iter, ops::Not, path::PathBuf, sync::Arc};

use async_recursion::async_recursion;

use super::{file_tree_like, parse_md5_digest, FileLike, FileTreeLike, FolderInfoLike, FolderLike};
use crate::{
    common::{drive_file, file_tree_drive::errors::FileIdentifier},
    files::list::{self, ListQuery, ListSortOrder},
    hub::Hub,
};

#[derive(Debug, Clone)]
pub struct FileTreeDrive {
    pub root: Folder,
}

impl FileTreeDrive {
    pub async fn from_file(
        hub: &Hub,
        file: google_drive3::api::File,
    ) -> Result<FileTreeDrive, errors::FileTreeDrive> {
        let root = Folder::from_file(hub, file, None)
            .await
            .map_err(errors::FileTreeDrive)?;
        Ok(FileTreeDrive { root })
    }
}

impl FileTreeLike for FileTreeDrive {
    type Folder = Folder;

    fn root(&self) -> &Self::Folder {
        &self.root
    }
}

type Node = file_tree_like::Node<Folder>;

#[derive(Debug, Clone)]
pub struct Folder {
    pub info: Arc<FolderInfo>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct FolderInfo {
    pub name: String,
    pub parent: Option<Arc<Self>>,
    pub drive_id: String,
}

impl Folder {
    #[async_recursion]
    pub async fn from_file(
        hub: &Hub,
        file: google_drive3::api::File,
        parent: Option<&'async_recursion Arc<FolderInfo>>,
    ) -> Result<Folder, errors::Folder> {
        if drive_file::is_directory(&file).not() {
            return Err(errors::Folder::NotDirectory);
        }

        let name = file.name.ok_or(errors::Folder::MissingFileName)?;
        let file_id = file.id.ok_or(errors::Folder::MissingFileId)?;

        let mut folder = Folder {
            info: Arc::new(FolderInfo {
                name,
                parent: parent.map(Arc::clone),
                drive_id: file_id.clone(),
            }),
            children: Vec::new(),
        };

        let files = list::list_files(
            hub,
            list::ListFilesConfig {
                query: &ListQuery::FilesInFolder { folder_id: file_id },
                order_by: &ListSortOrder::default(),
                max_files: usize::MAX,
            },
        )
        .await
        .map_err(errors::Folder::ListFiles)?;

        let mut children = Vec::new();

        for file in files {
            if drive_file::is_directory(&file) {
                let folder = Folder::from_file(hub, file, Some(&folder.info)).await?;
                let node = Node::Folder(folder);
                children.push(node);
            } else if drive_file::is_binary(&file) {
                let f = File::from_file(file, &folder)
                    .map_err(|(source, identifier)| errors::Folder::File { identifier, source })?;
                let node = Node::File(f);
                children.push(node);
            } else {
                // Skip documents
            }
        }

        folder.children = children;

        Ok(folder)
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
    #[must_use]
    pub fn relative_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        for folder in self.ancestors() {
            path.push(&folder.name);
        }

        path.join(&self.name)
    }

    fn ancestors(&self) -> Vec<Arc<FolderInfo>> {
        let mut folders = iter::successors(self.parent.as_ref(), |folder_info| {
            folder_info.parent.as_ref()
        })
        .cloned()
        .collect::<Vec<_>>();

        folders.reverse();
        folders
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
    pub size: u64,
    pub parent: Arc<FolderInfo>,
    pub drive_id: String,
    pub md5: Option<md5::Digest>,
}

impl File {
    pub fn from_file(
        file: google_drive3::api::File,
        parent: &Folder,
    ) -> Result<File, (errors::File, FileIdentifier)> {
        let name = file
            .name
            .ok_or((errors::File::MissingFileName, FileIdentifier::None))?;
        let Some(size) = file.size else {
            return Err((errors::File::MissingFileSize, FileIdentifier::Name(name)));
        };
        let size = match size.try_into() {
            Ok(size) => size,
            Err(source) => {
                return Err((
                    errors::File::InvalidFileSize(source),
                    FileIdentifier::Name(name),
                ))
            }
        };
        let Some(file_id) = file.id else {
            return Err((errors::File::MissingFileId, FileIdentifier::Name(name)));
        };
        let md5 = file.md5_checksum.as_deref().and_then(parse_md5_digest);

        let file = File {
            name,
            size,
            parent: Arc::clone(&parent.info),
            drive_id: file_id,
            md5,
        };

        Ok(file)
    }

    #[must_use]
    pub fn relative_path(&self) -> PathBuf {
        self.parent.relative_path().join(&self.name)
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::FolderInfo;

    #[test]
    fn folder_info_ancestors() {
        let folder_a = Arc::new(FolderInfo {
            name: "a".to_string(),
            parent: None,
            drive_id: "a".to_string(),
        });
        let folder_b = Arc::new(FolderInfo {
            name: "b".to_string(),
            parent: Some(Arc::clone(&folder_a)),
            drive_id: "b".to_string(),
        });
        let folder_c = Arc::new(FolderInfo {
            name: "c".to_string(),
            parent: Some(Arc::clone(&folder_b)),
            drive_id: "c".to_string(),
        });
        let folder_d = Arc::new(FolderInfo {
            name: "d".to_string(),
            parent: Some(Arc::clone(&folder_c)),
            drive_id: "d".to_string(),
        });

        let ancestors = folder_d.ancestors();
        let ancestors = ancestors.iter().map(|folder| folder.name.as_str());
        assert!(ancestors.eq(["a", "b", "c"]));
    }
}
