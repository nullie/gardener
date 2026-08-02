use rootcause::Result;

use crate::{declarative, fs};

pub trait Reporter {
    fn report_file(
        &mut self,
        path: std::path::PathBuf,
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
    fn visit_dir(
        &mut self,
        _path: std::path::PathBuf,
        maybe_declared: std::option::Option<(
            declarative::PathType,
            std::option::Option<declarative::Properties<'a>>,
        )>,
        has_declared_children: bool,
    ) -> bool {
        let maybe_properties =
            maybe_declared.and_then(|(_declared_path_type, maybe_properties)| maybe_properties);

        has_declared_children
            || maybe_properties.is_none_or(|properties| properties.storage_class.should_backup())
    }

    fn visit_file(
        &mut self,
        path: std::path::PathBuf,
        file_type: fs::FileType,
        len: u64,
        maybe_declared: std::option::Option<(
            declarative::PathType,
            std::option::Option<declarative::Properties<'a>>,
        )>,
    ) {
        let maybe_properties =
            maybe_declared.and_then(|(_declared_path_type, maybe_properties)| maybe_properties);

        if maybe_properties.is_none_or(|properties| properties.storage_class.should_backup()) {
            self.reporter
                .report_file(path, file_type, len, maybe_properties);
        }
    }

    fn visit_error(&mut self, dir: std::path::PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}
