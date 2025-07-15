use std::iter;

pub(super) trait FolderLike {
    fn parent(&self) -> Option<&Self>;

    #[must_use]
    fn ancestor_count(&self) -> usize {
        iter::successors(self.parent(), |folder| folder.parent()).count()
    }
}

#[cfg(test)]
mod test {
    use super::FolderLike;

    #[derive(Debug, Default)]
    struct Folder {
        parent: Option<Box<Folder>>,
    }

    impl FolderLike for Folder {
        fn parent(&self) -> Option<&Self> {
            self.parent.as_deref()
        }
    }

    #[test]
    fn ancestor_count() {
        let folder = Folder {
            parent: Some(Box::new(Folder {
                parent: Some(Box::new(Folder::default())),
            })),
        };

        assert_eq!(folder.ancestor_count(), 2);
    }
}
