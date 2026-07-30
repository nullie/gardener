use std::path::Path;

use eyre::Result;

use crate::{
    config::Config,
    declarative::{tmpfiles::add_systemd_tmpfiles, tree::Tree},
    fs,
    presentation::UntrackedPath,
};

pub fn ls() -> Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    fs::visit_dirs(Path::new("/"), &tree, &mut LsVisitor {})
}

struct LsVisitor {}

impl LsVisitor {}

impl<'a> fs::Visitor<'a> for LsVisitor {
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
            println!(
                "{}: {} {:?}",
                UntrackedPath::new(path, file_type),
                len,
                maybe_properties.map(|properties| properties.owner)
            );
        }
    }

    fn visit_error(&mut self, dir: std::path::PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}

pub fn size() -> Result<()> {
    todo!()
}
