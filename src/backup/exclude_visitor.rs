use std::{ops::ControlFlow, path::Path};

use rootcause::Result;

use crate::{
    decl::{self, Props},
    fs::unix::{self, walker},
};

pub trait Reporter {
    fn exclude(&mut self, path: &std::path::Path, path_type: walker::PathType);
}

pub struct Visitor<R: Reporter> {
    reporter: R,
}

impl<R: Reporter> Visitor<R> {
    pub fn new(reporter: R) -> Self {
        Self { reporter }
    }

    pub fn visit_fs(path: &std::path::Path, tree: &crate::decl::Tree, reporter: R) -> Result<R> {
        let mut visitor = Visitor::new(reporter);

        walker::walk_tree(path, tree, &mut visitor)?;

        Ok(visitor.reporter)
    }
}

impl<'a, R: Reporter> unix::walker::Visitor<Props<'a>> for Visitor<R> {
    // TODO: one visit method with Entry<DirProps, FileProps>
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: decl::Entry<Props>,
        recurse: impl FnOnce() -> Box<dyn Iterator<Item = bool>>,
        has_declared_children: bool,
    ) -> bool {
        let path_type = walker::PathType::Dir(());
        let matches_declared = path_type.matches(declared.maybe_path_type);
        let should_backup_this = !matches_declared
            || declared
                .maybe_props
                .is_none_or(|props| props.storage_class.should_backup());

        if !matches_declared {
            true
        } else {
            let has_backups_inside =
                has_declared_children && recurse().any(|should_backup| should_backup);
            if !has_backups_inside && !should_backup_this {
                self.reporter.exclude(path, path_type);
                false
            } else {
                true
            }
        }
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: unix::FileType,
        declared: decl::Entry<Props>,
        _len: u64,
    ) {
        let path_type = walker::PathType::File(file_type);

        let matches_declared = path_type.matches(declared.maybe_path_type);

        let should_backup = !matches_declared
            || declared
                .maybe_props
                .is_none_or(|props| props.storage_class.should_backup());

        if !should_backup {
            self.reporter.exclude(path, path_type);
        }
    }

    fn visit_error(&mut self, dir: &std::path::Path, e: std::io::Error) {
        eprintln!("Failed to read dir {:?}: {}", dir, e);
    }
}
