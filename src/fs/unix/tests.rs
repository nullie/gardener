use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use crate::{
    decl::{self},
    fs::{self, TypeChar},
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
    test_tree.file("/non-owning/intermediate/tracked", "non-owning");
    test_tree.dir("/owning", true, "owning");
    test_tree.dir("/owning/inside-dir/nested-owning", true, "nested-owning");
    test_tree.file("/non-dir/inside-non-dir", "inside-non-dir");

    let mut visitor = TestVisitor::new(test_dir.tempdir.path());
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

struct TestVisitor<'a, P> {
    root: &'a Path,
    visits: Vec<TestVisit<P>>,
}

impl<'a, P: Eq> TestVisitor<'a, P> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            visits: Vec::new(),
        }
    }

    fn report_visit(&mut self, path: &Path, visit: TestVisitProps, declared: decl::Entry<P>) {
        self.visits.push(TestVisit {
            path: Path::new("/").join(path.strip_prefix(self.root).expect("path not in root")),
            visit,
            declared,
        });
    }
}

#[derive(PartialEq, Eq)]
struct TestVisit<P> {
    path: PathBuf,
    visit: TestVisitProps,
    declared: decl::Entry<P>,
}

impl std::fmt::Debug for TestVisit<&str> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)?;

        match self.visit {
            fs::Entry::Dir(d) => write!(f, ".visit().dir({:?})", d.has_declared_children)?,
            fs::Entry::File(file_visit) => write!(
                f,
                ".visit({:?}).file({:?})",
                file_visit.file_type, file_visit.len
            )?,
        }

        if let Some(path_type) = self.declared.maybe_path_type {
            match path_type {
                fs::Entry::Dir(d) => write!(f, ".declared().dir({:?})", d.owns_contents)?,
                fs::Entry::File(file_type) => write!(f, ".declared().file({file_type:?})")?,
            }
        }

        if let Some(props) = self.declared.maybe_props {
            write!(f, ".props({:?})", props)?;
        }

        Ok(())
    }
}

type TestVisitProps = fs::Entry<DirVisit, FileVisit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirVisit {
    has_declared_children: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileVisit {
    file_type: super::FileType,
    len: u64,
}

impl TypeChar for FileVisit {
    fn type_char(&self) -> char {
        self.file_type.type_char()
    }
}

impl<P: Eq> fs::unix::walker::Visitor<P> for TestVisitor<'_, P> {
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: decl::Entry<P>,
        has_declared_children: bool,
    ) -> std::ops::ControlFlow<(), ()> {
        self.report_visit(
            path,
            TestVisitProps::Dir(DirVisit {
                has_declared_children,
            }),
            declared,
        );

        std::ops::ControlFlow::Continue(())
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: super::FileType,
        declared: decl::Entry<P>,
        len: u64,
    ) {
        self.report_visit(
            path,
            TestVisitProps::File(FileVisit { file_type, len }),
            declared,
        );
    }

    fn visit_error(&mut self, _dir: &Path, _e: std::io::Error) {
        panic!("Unexpected error in visitor")
    }
}

trait TestVisitEntry: Sized {
    fn visit(&'_ self) -> VisitBuilder<'_>;
}

impl<P> TestVisitEntry for P
where
    P: AsRef<Path>,
{
    fn visit(&'_ self) -> VisitBuilder<'_> {
        VisitBuilder {
            path: self.as_ref(),
        }
    }
}

struct VisitBuilder<'a> {
    path: &'a Path,
}

impl<'a> VisitBuilder<'a> {
    fn dir(self, has_declared_children: bool) -> (&'a Path, TestVisitProps) {
        (
            self.path,
            TestVisitProps::Dir(DirVisit {
                has_declared_children,
            }),
        )
    }

    fn file(self, file_type: super::FileType, len: u64) -> (&'a Path, TestVisitProps) {
        (
            self.path,
            TestVisitProps::File(FileVisit { file_type, len }),
        )
    }
}
