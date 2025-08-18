pub mod errors;

use crate::common::drive_file;
use crate::common::file_tree_drive::errors::FileIdentifier;
use crate::files::list;
use crate::files::list::ListQuery;
use crate::files::list::ListSortOrder;
use crate::hub::Hub;
use async_recursion::async_recursion;
use std::ops::Not;
use std::path::PathBuf;

use super::parse_md5_digest;
use super::FolderLike;

#[derive(Debug, Clone)]
pub struct FileTreeDrive {
    pub root: Folder,
}

impl FileTreeDrive {
    pub async fn from_file(
        hub: &Hub,
        file: &google_drive3::api::File,
    ) -> Result<FileTreeDrive, errors::FileTreeDrive> {
        let root = Folder::from_file(hub, file, None)
            .await
            .map_err(errors::FileTreeDrive)?;
        Ok(FileTreeDrive { root })
    }

    #[must_use]
    pub fn folders(&self) -> Vec<&Folder> {
        let mut folders = vec![];

        folders.push(&self.root);
        self.root.folders_recursive_in(&mut folders);

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
    pub parent: Option<Box<Folder>>,
    pub children: Vec<Node>,
    pub drive_id: String,
}

impl Folder {
    #[async_recursion]
    pub async fn from_file(
        hub: &Hub,
        file: &google_drive3::api::File,
        parent: Option<&'async_recursion Folder>,
    ) -> Result<Folder, errors::Folder> {
        if drive_file::is_directory(file).not() {
            return Err(errors::Folder::NotDirectory);
        }

        let name = file.name.clone().ok_or(errors::Folder::MissingFileName)?;
        let file_id = file.id.clone().ok_or(errors::Folder::MissingFileId)?;

        let mut folder = Folder {
            name,
            parent: parent.map(|folder| Box::new(folder.clone())),
            children: Vec::new(),
            drive_id: file_id.clone(),
        };

        let files = list::list_files(
            hub,
            &list::ListFilesConfig {
                query: ListQuery::FilesInFolder { folder_id: file_id },
                order_by: ListSortOrder::default(),
                max_files: usize::MAX,
            },
        )
        .await
        .map_err(errors::Folder::ListFiles)?;

        let mut children = Vec::new();

        for file in files {
            if drive_file::is_directory(&file) {
                let folder = Folder::from_file(hub, &file, Some(&folder)).await?;
                let node = Node::FolderNode(folder);
                children.push(node);
            } else if drive_file::is_binary(&file) {
                let f = File::from_file(&file, &folder).map_err(|source| errors::Folder::File {
                    identifier: FileIdentifier::from(file),
                    source,
                })?;
                let node = Node::FileNode(f);
                children.push(node);
            } else {
                // Skip documents
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
    pub fn relative_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        for folder in get_ancestors(self) {
            path.push(&folder.name);
        }

        path.join(&self.name)
    }

    #[must_use]
    pub fn folders_recursive(&self) -> Vec<&Folder> {
        let mut folders = vec![];
        self.folders_recursive_in(&mut folders);
        folders
    }

    fn folders_recursive_in<'a>(&'a self, folders: &mut Vec<&'a Folder>) {
        self.children.iter().for_each(|child| {
            if let Node::FolderNode(folder) = child {
                folders.push(folder);
                folder.folders_recursive_in(folders);
            }
        });
    }

    #[must_use]
    pub fn ancestor_count(&self) -> usize {
        FolderLike::ancestor_count(self)
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
    pub size: u64,
    pub parent: Folder,
    pub drive_id: String,
    pub md5: Option<md5::Digest>,
}

impl File {
    pub fn from_file(
        file: &google_drive3::api::File,
        parent: &Folder,
    ) -> Result<File, errors::File> {
        let name = file.name.clone().ok_or(errors::File::MissingFileName)?;
        let size = file
            .size
            .ok_or(errors::File::MissingFileSize)?
            .try_into()
            .map_err(errors::File::InvalidFileSize)?;
        let file_id = file.id.clone().ok_or(errors::File::MissingFileId)?;
        let md5 = file.md5_checksum.as_deref().and_then(parse_md5_digest);

        let file = File {
            name,
            size,
            parent: parent.clone(),
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

fn get_ancestors(f: &Folder) -> Vec<Folder> {
    let mut folders = Vec::new();
    let mut maybe_folder = f.parent.clone();

    while let Some(folder) = maybe_folder {
        folders.push(*folder.clone());

        if folder.parent.is_none() {
            break;
        }

        maybe_folder = folder.parent;
    }

    folders.reverse();
    folders
}
