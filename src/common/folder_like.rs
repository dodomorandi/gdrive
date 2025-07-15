use std::iter;

pub(super) trait FolderLike {
    fn parent(&self) -> Option<&Self>;

    #[must_use]
    fn ancestor_count(&self) -> usize {
        iter::successors(self.parent(), |folder| folder.parent()).count()
    }
}
