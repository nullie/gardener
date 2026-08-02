use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

use crate::declarative::{
    self,
    tree::{Node, Tree},
};

pub trait Visitor<'a> {
    fn visit_dir(
        &mut self,
        path: PathBuf,
        maybe_declared: Option<(declarative::PathType, Option<declarative::Properties<'a>>)>,
        has_declared_children: bool,
    ) -> bool;
    fn visit_file(
        &mut self,
        path: PathBuf,
        file_type: FileType,
        len: u64,
        maybe_declared: Option<(declarative::PathType, Option<declarative::Properties<'a>>)>,
    );
    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error);
}

pub fn walk_tree<'a>(
    dir: &Path,
    tree: &'a Tree<'a>,
    visitor: &mut impl Visitor<'a>,
) -> rootcause::Result<()> {
    walk_dir(dir, Some(&tree.root), None, visitor).map_err(rootcause::Report::from)
}

fn walk_dir<'a>(
    dir: &Path,
    maybe_tree_directory: Option<&'a BTreeMap<OsString, Node>>,
    inherited_properties: Option<declarative::Properties<'a>>,
    visitor: &mut impl Visitor<'a>,
) -> Result<(), std::io::Error> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry_result in entries {
                match entry_result {
                    Ok(entry) => {
                        match process_entry(
                            &entry,
                            maybe_tree_directory,
                            inherited_properties,
                            visitor,
                        ) {
                            Ok(()) => (),
                            Err(e) => visitor.visit_error(entry.path(), e),
                        };
                    }
                    Err(e) => visitor.visit_error(dir.to_owned(), e),
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir.to_owned(), e);
        }
    }

    Ok(())
}

fn process_entry<'a>(
    entry: &std::fs::DirEntry,
    maybe_tree_directory: Option<&'a BTreeMap<OsString, Node>>,
    inherited_properties: Option<declarative::Properties<'a>>,
    visitor: &mut impl Visitor<'a>,
) -> Result<(), std::io::Error> {
    let metadata = entry.metadata()?;
    let path = entry.path();
    let path_type = PathType::from(entry.file_type()?);
    let maybe_tree_node = maybe_tree_directory
        .and_then(|tree_directory| tree_directory.get(entry.file_name().as_os_str()));
    let maybe_declared =
        maybe_tree_node.map(|tree_node| (tree_node.path_type(), tree_node.get_properties()));

    let maybe_properties = maybe_tree_node
        .and_then(|tree_node| tree_node.get_properties())
        .or(inherited_properties);
    let maybe_children = maybe_tree_node.and_then(|tree_node| tree_node.get_children());

    match path_type {
        PathType::Directory => {
            if visitor.visit_dir(
                path.clone(),
                maybe_declared,
                maybe_children.is_some_and(|children| !children.is_empty()),
            ) {
                walk_dir(&path, maybe_children, maybe_properties, visitor)?;
            }
        }
        PathType::File(file_type) => {
            visitor.visit_file(path, file_type, metadata.len(), maybe_declared);
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    Declarative(declarative::FileType),
    Other(fs::FileType),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PathType {
    Directory,
    File(FileType),
}

impl From<std::fs::FileType> for PathType {
    fn from(std_file_type: std::fs::FileType) -> Self {
        if std_file_type.is_dir() {
            Self::Directory
        } else {
            let file_type = if std_file_type.is_file() {
                FileType::Declarative(declarative::FileType::Regular)
            } else if std_file_type.is_symlink() {
                FileType::Declarative(declarative::FileType::Symlink)
            } else if std_file_type.is_char_device() {
                FileType::Declarative(declarative::FileType::CharDevice)
            } else if std_file_type.is_block_device() {
                FileType::Declarative(declarative::FileType::BlockDevice)
            } else if std_file_type.is_fifo() {
                FileType::Declarative(declarative::FileType::Fifo)
            } else {
                FileType::Other(std_file_type)
            };

            Self::File(file_type)
        }
    }
}

impl From<declarative::FileType> for FileType {
    fn from(file_type: declarative::FileType) -> Self {
        Self::Declarative(file_type)
    }
}

impl From<declarative::PathType> for PathType {
    fn from(path_type: declarative::PathType) -> Self {
        match path_type {
            declarative::PathType::Directory { .. } => Self::Directory,
            declarative::PathType::File(file_type) => Self::File(file_type.into()),
        }
    }
}

impl From<FileType> for PathType {
    fn from(file_type: FileType) -> Self {
        Self::File(file_type)
    }
}
