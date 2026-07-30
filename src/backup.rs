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
        path: std::path::PathBuf,
        maybe_owner: Option<crate::declarative::Owner<'a>>,
        expected: Option<fs::FileType>,
        has_declared_children: bool,
    ) -> bool {
        has_declared_children || maybe_owner.is_none_or(|owner| owner.should_backup())
    }

    fn visit_file(
        &mut self,
        path: std::path::PathBuf,
        owner: Option<crate::declarative::Owner<'a>>,
        file_type: fs::FileType,
        expected: Option<fs::FileType>,
    ) {
        if owner.is_none_or(|owner| owner.should_backup()) {
            println!("{}: {:?}", UntrackedPath::new(path, file_type), owner);
        }
    }

    fn visit_error(&mut self, dir: std::path::PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}

pub fn size() -> Result<()> {
    todo!()
}
