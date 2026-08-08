use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use crate::{
    declarative::{self},
    fs,
};

#[test]
fn test_visitor() {
    let test_directory = TestDirectory::new(&[
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
    super::walker::walk_tree(test_directory.path(), &test_tree.tree, &mut visitor)
        .expect("walk_tree failed");

    assert_eq!(visitor.visits, vec![]);
}

struct TestDirectory {
    tempdir: tempdir::TempDir,
}

impl TestDirectory {
    fn new(entries: &[&str]) -> Self {
        let tempdir = TempDir::new("gardener-test").expect("couldn't create temporary directory");

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
    tree: declarative::tree::Tree<P>,
}

impl<P: Copy + std::fmt::Debug> TestTree<P> {
    fn new() -> Self {
        Self {
            tree: declarative::tree::Tree::new(),
        }
    }

    fn dir(&mut self, path: &str, owns_contents: bool, properties: P) {
        let path_type =
            declarative::PathType::Directory(declarative::DirectoryProperties { owns_contents });

        self.tree
            .add_path(Path::new(path), path_type, properties)
            .expect("add_path failed")
    }

    fn file(&mut self, path: &str, properties: P) {
        let path_type = declarative::PathType::File(declarative::FileType::Regular);

        self.tree
            .add_path(Path::new(path), path_type, properties)
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
    declared: declarative::Entry<P>,
}

type VisitKind = fs::Entry<DirectoryVisit, FileVisit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryVisit {
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
        declared: declarative::Entry<P>,
        has_declared_children: bool,
    ) -> std::ops::ControlFlow<(), ()> {
        self.visits.push(TestVisit {
            path: path.to_owned(),
            kind: VisitKind::Directory(DirectoryVisit {
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
        declared: declarative::Entry<P>,
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
