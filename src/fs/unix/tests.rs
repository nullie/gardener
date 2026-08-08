use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use crate::{
    decl::{self},
    fs,
};

#[test]
fn test_visitor() {
    let test_dir = TestDir::new(&[
        "conflicting-type-file",
        "conflicting-type-dir/",
        "non-owning/",
        "non-owning/untracked",
        "non-owning/intermediate/tracked",
        "owning/",
        "owning/inside-dir/",
        "owning/inside-dir/nested-owning/",
        "owning/inside-dir/nested-owning/tracked",
        "non-dir",
        "untracked/",
        "untracked/untracked",
    ]);

    let mut test_tree = TestTree::<&str>::new();

    test_tree.dir("/conflicting-type-file", true, "conflicting");
    test_tree.file("/conflicting-type-dir", "conflicting");
    test_tree.dir("/non-owning", false, "non-owning");
    test_tree.dir("/owning", true, "owning");
    test_tree.dir("/owning/inside-dir/nested-owning", true, "neseted-owning");
    test_tree.file("/non-dir/inside-non-dir", "inside-non-dir");

    let mut visitor = TestVisitor::new();
    super::walker::walk_tree(test_dir.path(), &test_tree.tree, &mut visitor)
        .expect("walk_tree failed");

    assert_eq!(visitor.visits, vec![]);
}

struct TestDir {
    tempdir: tempdir::TempDir,
}

impl TestDir {
    fn new(entries: &[&str]) -> Self {
        let tempdir = TempDir::new("gardener-test").expect("couldn't create temporary dir");

        for &entry in entries {
            let (path, is_dir) = if let Some(without_suffix) = entry.strip_suffix('/') {
                (without_suffix, true)
            } else {
                (entry, false)
            };

            let path = tempdir.path().join(path);

            if is_dir {
                std::fs::create_dir_all(path).expect("failed to create dir");
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).expect("failed to create parent dir");
                }
                std::fs::File::create_new(path).expect("failed to create file");
            }
        }

        Self { tempdir }
    }

    fn path(&self) -> &Path {
        self.tempdir.path()
    }
}

struct TestTree<P> {
    tree: decl::tree::Tree<P>,
}

impl<P: Copy + std::fmt::Debug> TestTree<P> {
    fn new() -> Self {
        Self {
            tree: decl::tree::Tree::new(),
        }
    }

    fn dir(&mut self, path: &str, owns_contents: bool, props: P) {
        let path_type = decl::PathType::Dir(decl::DirProps { owns_contents });

        self.tree
            .add_path(Path::new(path), path_type, props)
            .expect("add_path failed")
    }

    fn file(&mut self, path: &str, props: P) {
        let path_type = decl::PathType::File(decl::FileType::Regular);

        self.tree
            .add_path(Path::new(path), path_type, props)
            .expect("add_path failed")
    }
}

struct TestVisitor<P> {
    visits: Vec<TestVisit<P>>,
}

impl<P: Eq> TestVisitor<P> {
    fn new() -> Self {
        Self { visits: Vec::new() }
    }
}

#[derive(PartialEq, Eq, Debug)]
struct TestVisit<P> {
    path: PathBuf,
    kind: VisitKind,
    declared: decl::Entry<P>,
}

type VisitKind = fs::Entry<DirVisit, FileVisit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirVisit {
    has_declared_children: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileVisit {
    file_type: super::FileType,
    len: u64,
}

impl<P: Eq> fs::unix::walker::Visitor<P> for TestVisitor<P> {
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: decl::Entry<P>,
        has_declared_children: bool,
    ) -> std::ops::ControlFlow<(), ()> {
        self.visits.push(TestVisit {
            path: path.to_owned(),
            kind: VisitKind::Dir(DirVisit {
                has_declared_children,
            }),
            declared,
        });

        std::ops::ControlFlow::Continue(())
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: super::FileType,
        declared: decl::Entry<P>,
        len: u64,
    ) {
        self.visits.push(TestVisit {
            path: path.to_owned(),
            kind: VisitKind::File(FileVisit { file_type, len }),
            declared,
        });
    }

    fn visit_error(&mut self, _dir: &Path, _e: std::io::Error) {
        panic!("Unexpected error in visitor")
    }
}
