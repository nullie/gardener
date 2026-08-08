use std::{ops::ControlFlow, path::Path};

use rootcause::Result;

use crate::{
    declarative::{self, Properties},
    fs::unix,
};

pub trait Reporter {
    fn report_file(
        &mut self,
        path: &std::path::Path,
        file_type: unix::FileType,
        len: u64,
        maybe_properties: Option<Properties>,
    );
}

pub struct Visitor<R: Reporter> {
    reporter: R,
}

impl<R: Reporter> Visitor<R> {
    pub fn new(reporter: R) -> Self {
        Self { reporter }
    }

    pub fn visit_fs(
        path: &std::path::Path,
        tree: &crate::declarative::Tree,
        reporter: R,
    ) -> Result<R> {
        let mut visitor = Visitor::new(reporter);

        unix::walker::walk_tree(path, tree, &mut visitor)?;

        Ok(visitor.reporter)
    }
}

impl<'a, R: Reporter> unix::walker::Visitor<Properties<'a>> for Visitor<R> {
    fn visit_dir(
        &mut self,
        _path: &Path,
        declared: declarative::Entry<Properties>,
        has_declared_children: bool,
    ) -> ControlFlow<(), ()> {
        if has_declared_children
            || declared
                .maybe_properties
                .is_none_or(|properties| properties.storage_class.should_backup())
        {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: unix::FileType,
        declared: declarative::Entry<Properties>,
        len: u64,
    ) {
        if declared
            .maybe_properties
            .is_none_or(|properties| properties.storage_class.should_backup())
        {
            self.reporter
                .report_file(path, file_type, len, declared.maybe_properties);
        }
    }

    fn visit_error(&mut self, dir: &std::path::Path, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}
