use std::{ops::ControlFlow, path::Path};

use rootcause::Result;

use crate::{
    decl::{self, Props},
    fs::unix::walker,
};

pub trait Reporter {
    fn report_file(
        &mut self,
        path: &std::path::Path,
        file_type: walker::FileType,
        len: u64,
        maybe_props: Option<Props>,
    );
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

impl<'a, R: Reporter> walker::Visitor<Props<'a>> for Visitor<R> {
    fn visit_dir(
        &mut self,
        _path: &Path,
        declared: decl::Entry<Props>,
        has_declared_children: bool,
    ) -> ControlFlow<(), ()> {
        if has_declared_children
            || declared
                .maybe_props
                .is_none_or(|props| props.storage_class.should_backup())
        {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: walker::FileType,
        declared: decl::Entry<Props>,
        len: u64,
    ) {
        if declared
            .maybe_props
            .is_none_or(|props| props.storage_class.should_backup())
        {
            self.reporter
                .report_file(path, file_type, len, declared.maybe_props);
        }
    }

    fn visit_error(&mut self, dir: &std::path::Path, e: std::io::Error) {
        eprintln!("Failed to read dir {:?}: {}", dir, e);
    }
}
