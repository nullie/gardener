use std::path::Path;

use crate::decl;

pub struct TestTree<P> {
    tree: decl::tree::Tree<P>,
}

impl<P: Copy + std::fmt::Debug> TestTree<P> {
    pub fn new() -> Self {
        Self {
            tree: decl::tree::Tree::new(),
        }
    }

    pub fn dir(&mut self, path: &str, owns_contents: bool, props: P) {
        let path_type = decl::PathType::Dir(decl::DirProps { owns_contents });

        self.tree
            .add_path(Path::new(path), path_type, props)
            .expect("add_path failed")
    }

    pub fn file(&mut self, path: &str, props: P) {
        let path_type = decl::PathType::File(decl::FileType::Regular);

        self.tree
            .add_path(Path::new(path), path_type, props)
            .expect("add_path failed")
    }
}
