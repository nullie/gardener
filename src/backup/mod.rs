mod file_visitor;

use std::path::Path;

use rootcause::Result;

use crate::fs;
use crate::{
    config::Config,
    declarative::{tmpfiles::add_systemd_tmpfiles, tree::Tree},
    presentation::UntrackedPath,
};

pub fn ls() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    file_visitor::Visitor::visit_fs(Path::new("/"), &tree, LsReporter)?;

    Ok(())
}

struct LsReporter;

impl file_visitor::Reporter for LsReporter {
    fn report_file(
        &mut self,
        path: std::path::PathBuf,
        file_type: fs::PathType,
        len: u64,
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
}

pub fn size() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let reporter = file_visitor::Visitor::visit_fs(Path::new("/"), &tree, SizeReporter::new())?;

    println!(
        "Backup size: {}",
        humansize::SizeFormatter::new(reporter.size, humansize::BINARY)
    );

    Ok(())
}

struct SizeReporter {
    size: u64,
}

impl SizeReporter {
    fn new() -> Self {
        Self { size: 0 }
    }
}

impl file_visitor::Reporter for SizeReporter {
    fn report_file(
        &mut self,
        _path: std::path::PathBuf,
        _file_type: fs::PathType,
        len: u64,
        _maybe_properties: Option<crate::declarative::Properties>,
    ) {
        self.size += len;
    }
}
