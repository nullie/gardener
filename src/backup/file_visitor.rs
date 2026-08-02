use std::ops::ControlFlow;

use rootcause::Result;

use crate::fs;

pub trait Reporter {
    fn report_file(
        &mut self,
        path: &std::path::Path,
        file_type: fs::FileType,
        len: u64,
        maybe_properties: Option<crate::declarative::Properties>,
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
        tree: &crate::declarative::tree::Tree,
        reporter: R,
    ) -> Result<R> {
        let mut visitor = Visitor::new(reporter);

        fs::walk_tree(path, tree, &mut visitor)?;

        Ok(visitor.reporter)
    }
}

impl<'a, R: Reporter> fs::Visitor<'a> for Visitor<R> {
    fn visit_dir(&mut self, entry: fs::Entry, has_declared_children: bool) -> ControlFlow<(), ()> {
        if has_declared_children
            || entry
                .maybe_properties
                .is_none_or(|properties| properties.storage_class.should_backup())
        {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }

    fn visit_file(&mut self, entry: fs::Entry, file_type: fs::FileType, len: u64) {
        if entry
            .maybe_properties
            .is_none_or(|properties| properties.storage_class.should_backup())
        {
            self.reporter
                .report_file(entry.path, file_type, len, entry.maybe_properties);
        }
    }

    fn visit_error(&mut self, dir: &std::path::Path, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}
