mod exclude_visitor;
mod file_visitor;

use std::path::Path;

use rootcause::Result;

use crate::{
    config::Config,
    decl::Props,
    fs::{
        self,
        unix::{self},
    },
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
        path: &std::path::Path,
        file_type: unix::FileType,
        len: u64,
        maybe_props: Option<Props>,
    ) {
        if maybe_props.is_none_or(|props| props.storage_class.should_backup()) {
            println!(
                "{}: {} {:?}",
                UntrackedPath::new(path, fs::Entry::File(file_type)),
                len,
                maybe_props
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
        _path: &std::path::Path,
        _file_type: unix::FileType,
        len: u64,
        _maybe_props: Option<Props>,
    ) {
        self.size += len;
    }
}

pub fn exclude() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    exclude_visitor::Visitor::visit_fs(Path::new("/"), &tree, ExcludeReporter)?;

    Ok(())
}

struct ExcludeReporter;

impl exclude_visitor::Reporter for ExcludeReporter {
    fn exclude(&mut self, path: &std::path::Path, path_type: unix::walker::PathType) {
        println!("{}", UntrackedPath::new(path, path_type));
    }
}

#[cfg(test)]
mod tests;
