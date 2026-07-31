use rootcause::Result;

use crate::fs;

pub trait Reporter {
    fn report_file(
        &mut self,
        path: std::path::PathBuf,
        file_type: fs::PathType,
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

        fs::visit_dirs(path, tree, &mut visitor)?;

        Ok(visitor.reporter)
    }
}

impl<'a, R: Reporter> fs::Visitor<'a> for Visitor<R> {
    fn visit_dir(
        &mut self,
        _path: std::path::PathBuf,
        _maybe_expected_path_type: Option<fs::PathType>,
        maybe_properties: Option<crate::declarative::Properties>,
        has_declared_children: bool,
    ) -> bool {
        has_declared_children
            || maybe_properties.is_none_or(|properties| properties.storage_class.should_backup())
    }

    fn visit_file(
        &mut self,
        path: std::path::PathBuf,
        file_type: fs::PathType,
        len: u64,
        _maybe_expected_path_type: Option<fs::PathType>,
        maybe_properties: Option<crate::declarative::Properties>,
    ) {
        if maybe_properties.is_none_or(|properties| properties.storage_class.should_backup()) {
            self.reporter
                .report_file(path, file_type, len, maybe_properties);
        }
    }

    fn visit_error(&mut self, dir: std::path::PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}
