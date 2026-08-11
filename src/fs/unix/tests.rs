use std::path::Path;

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use crate::decl::{self};

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

    let mut visitor = test_visits::TestVisitor::new(test_dir.tempdir.path());
    super::walker::walk_tree(test_dir.path(), &test_tree.tree, &mut visitor)
        .expect("walk_tree failed");

    {
        use test_visits::prelude::*;

        assert_eq!(
            visitor.visits,
            vec![
                "/non-dir".visit_file(Regular, 0).declared_dir(false),
                "/untracked".visit_dir(false),
                "/untracked/untracked".visit_file(Regular, 0),
                "/conflicting-type-dir"
                    .visit_dir(false)
                    .declared_file(Regular)
                    .props("conflicting"),
                "/non-owning"
                    .visit_dir(true)
                    .declared_dir(false)
                    .props("non-owning"),
                "/non-owning/untracked".visit_file(Regular, 0),
                "/non-owning/intermediate"
                    .visit_dir(true)
                    .declared_dir(false),
                "/non-owning/intermediate/tracked"
                    .visit_file(Regular, 0)
                    .declared_file(Regular)
                    .props("non-owning"),
                "/conflicting-type-file"
                    .visit_file(Regular, 0)
                    .declared_dir(true)
                    .props("conflicting"),
                "/owning".visit_dir(true).declared_dir(true).props("owning"),
                "/owning/inside-dir"
                    .visit_dir(true)
                    .declared_dir(false)
                    .props("owning"),
                "/owning/inside-dir/nested-owning"
                    .visit_dir(false)
                    .declared_dir(true)
                    .props("nested-owning"),
                "/owning/inside-dir/nested-owning/tracked"
                    .visit_file(Regular, 0)
                    .props("nested-owning"),
            ]
        );
    }
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

mod test_visits {
    use std::path::{Path, PathBuf};

    use crate::{
        decl::{self},
        fs::{
            self,
            TypeChar,
            unix::{self, walker},
        },
    };

    pub mod prelude {
        pub use super::TestVisitEntry;
        pub use crate::fs::unix::FileType::Regular;
    }

    pub struct TestVisitor<'a, P> {
        root: &'a Path,
        pub visits: Vec<TestVisit<P>>,
    }

    impl<'a, P: Eq> TestVisitor<'a, P> {
        pub fn new(root: &'a Path) -> Self {
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
    pub struct TestVisit<P> {
        path: PathBuf,
        visit: TestVisitProps,
        declared: decl::Entry<P>,
    }

    impl<P> TestVisit<P> {
        pub fn declared_dir(mut self, owns_contents: bool) -> Self {
            self.declared.maybe_path_type =
                Some(decl::PathType::Dir(decl::DirProps { owns_contents }));

            self
        }

        pub fn declared_file(mut self, file_type: decl::FileType) -> Self {
            self.declared.maybe_path_type = Some(decl::PathType::File(file_type));

            self
        }

        pub fn props(mut self, props: P) -> Self {
            self.declared.maybe_props = Some(props);

            self
        }
    }

    impl std::fmt::Debug for TestVisit<&str> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.path)?;

            match self.visit {
                fs::Entry::Dir(d) => write!(f, ".visit_dir({:?})", d.has_declared_children)?,
                fs::Entry::File(file_visit) => write!(
                    f,
                    ".visit_file({:?}, {:?})",
                    file_visit.file_type, file_visit.len
                )?,
            }

            if let Some(path_type) = self.declared.maybe_path_type {
                match path_type {
                    fs::Entry::Dir(d) => write!(f, ".declared_dir({:?})", d.owns_contents)?,
                    fs::Entry::File(file_type) => write!(f, ".declared_file({file_type:?})")?,
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
        file_type: unix::FileType<std::fs::FileType>,
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
            file_type: walker::FileType,
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

    pub trait TestVisitEntry {
        fn visit_dir<P>(&self, has_declared_children: bool) -> TestVisit<P>;
        fn visit_file<P>(&self, file_type: walker::FileType, len: u64) -> TestVisit<P>;
    }

    impl<P> TestVisitEntry for P
    where
        P: AsRef<std::path::Path> + ?Sized,
    {
        fn visit_dir<Props>(&self, has_declared_children: bool) -> TestVisit<Props> {
            TestVisit {
                path: self.as_ref().to_path_buf(),
                visit: TestVisitProps::Dir(DirVisit {
                    has_declared_children,
                }),
                declared: decl::Entry {
                    maybe_path_type: None,
                    maybe_props: None,
                },
            }
        }

        fn visit_file<Props>(&self, file_type: walker::FileType, len: u64) -> TestVisit<Props> {
            TestVisit {
                path: self.as_ref().to_path_buf(),
                visit: TestVisitProps::File(FileVisit { file_type, len }),
                declared: decl::Entry {
                    maybe_path_type: None,
                    maybe_props: None,
                },
            }
        }
    }
}
