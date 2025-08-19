use std::{iter, sync::Arc};

pub trait FileTreeLike: Sized {
    type Folder: FolderLike;

    #[must_use]
    fn root(&self) -> &Self::Folder;

    #[must_use]
    fn folders(&self) -> Vec<&Self::Folder> {
        let mut folders = vec![];

        folders.push(self.root());
        self.root().folders_recursive_in(&mut folders);

        folders.sort_by(|a, b| {
            let parent_count_a = a.info().ancestor_count();
            let parent_count_b = b.info().ancestor_count();

            parent_count_a
                .cmp(&parent_count_b)
                .then_with(|| a.info().name().cmp(b.info().name()))
        });

        folders
    }

    #[must_use]
    fn info(&self) -> TreeInfo {
        let mut file_count = 0;
        let mut folder_count = 0;
        let mut total_file_size = 0;

        for folder in self.folders() {
            folder_count += 1;

            for file in folder.files() {
                file_count += 1;
                total_file_size += file.size();
            }
        }

        TreeInfo {
            file_count,
            folder_count,
            total_file_size,
        }
    }
}

pub trait FolderLike: Sized {
    type File: FileLike;
    type Info: FolderInfoLike;

    #[must_use]
    fn children(&self) -> &[Node<Self>];

    #[must_use]
    fn info(&self) -> &Arc<Self::Info>;

    fn folders_recursive_in<'a>(&'a self, folders: &mut Vec<&'a Self>) {
        self.children().iter().for_each(|child| {
            if let Node::Folder(folder) = child {
                folders.push(folder);
                folder.folders_recursive_in(folders);
            }
        });
    }

    #[must_use]
    fn folders_recursive(&self) -> Vec<&Self> {
        let mut folders = vec![];
        self.folders_recursive_in(&mut folders);
        folders
    }

    #[must_use]
    fn files(&self) -> Vec<Self::File> {
        let mut files = vec![];

        for child in self.children() {
            if let Node::File(file) = child {
                files.push(file.clone());
            }
        }

        files.sort_by(|a, b| a.name().cmp(b.name()));

        files
    }
}

pub trait FolderInfoLike: Sized {
    #[must_use]
    fn name(&self) -> &str;

    #[must_use]
    fn parent(&self) -> Option<&Arc<Self>>;

    #[must_use]
    fn ancestor_count(&self) -> usize {
        iter::successors(self.parent(), |folder| folder.parent()).count()
    }
}

pub trait FileLike: Clone {
    #[must_use]
    fn name(&self) -> &str;

    #[must_use]
    fn size(&self) -> u64;
}

#[derive(Debug, Clone)]
pub enum Node<F: FolderLike> {
    Folder(F),
    File(F::File),
}

#[derive(Debug, Clone)]
pub struct TreeInfo {
    pub file_count: u64,
    pub folder_count: u64,
    pub total_file_size: u64,
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::FolderInfoLike;

    #[derive(Debug, Default)]
    struct FolderInfo {
        parent: Option<Arc<FolderInfo>>,
    }

    impl FolderInfoLike for FolderInfo {
        fn name(&self) -> &str {
            todo!()
        }

        fn parent(&self) -> Option<&Arc<Self>> {
            self.parent.as_ref()
        }
    }

    #[test]
    fn ancestor_count() {
        let folder = FolderInfo {
            parent: Some(Arc::new(FolderInfo {
                parent: Some(Arc::new(FolderInfo::default())),
            })),
        };

        assert_eq!(folder.ancestor_count(), 2);
    }
}
