use crate::common::drive_file;
use crate::files::list;
use crate::files::list::ListQuery;
use crate::files::list::ListSortOrder;
use crate::hub::Hub;
use async_recursion::async_recursion;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
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
    ) -> Result<FileTreeDrive, Error> {
        let root = Folder::from_file(hub, file, None).await?;
        Ok(FileTreeDrive { root })
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

            if parent_count_a == parent_count_b {
                a.name.cmp(&b.name)
            } else {
                parent_count_a.cmp(&parent_count_b)
            }
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
    ) -> Result<Folder, Error> {
        if drive_file::is_directory(file).not() {
            let name = file
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            return Err(Error::NotADirectory(name));
        }

        let name = file.name.clone().ok_or(Error::MissingFileName)?;
        let file_id = file.id.clone().ok_or(Error::MissingFileId)?;

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
        .map_err(Error::ListFiles)?;

        let mut children = Vec::new();

        for file in files {
            if drive_file::is_directory(&file) {
                let folder = Folder::from_file(hub, &file, Some(&folder)).await?;
                let node = Node::FolderNode(folder);
                children.push(node);
            } else if drive_file::is_binary(&file) {
                let f = File::from_file(&file, &folder)?;
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
    pub fn from_file(file: &google_drive3::api::File, parent: &Folder) -> Result<File, Error> {
        let name = file.name.clone().ok_or(Error::MissingFileName)?;
        let size = file
            .size
            .ok_or(Error::MissingFileSize)?
            .try_into()
            .map_err(|_| Error::InvalidFileSize)?;
        let file_id = file.id.clone().ok_or(Error::MissingFileId)?;
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

#[derive(Debug)]
pub enum Error {
    NotADirectory(String),
    MissingFileName,
    MissingFileId,
    MissingFileSize,
    InvalidFileSize,
    ListFiles(list::Error),
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotADirectory(name) => write!(f, "'{name}' is not a directory"),
            Error::MissingFileName => write!(f, "Drive file is missing file name"),
            Error::MissingFileId => write!(f, "Drive file is missing file id"),
            Error::MissingFileSize => write!(f, "Drive file is missing file size"),
            Error::InvalidFileSize => f.write_str("Drive file size is negative thus invalid"),
            Error::ListFiles(err) => write!(f, "Failed to list files: {err}"),
        }
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
