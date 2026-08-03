use std::{fs, ops::ControlFlow, os::unix::fs::FileTypeExt, path::Path};

use crate::declarative::{
    self,
    tree::{NodeChildren, Tree},
};

pub trait Visitor<'a> {
    fn visit_dir(
        &mut self,
        declared: Entry<'a, '_>,
        has_declared_children: bool,
    ) -> ControlFlow<(), ()>;
    fn visit_file(&mut self, declared: Entry<'a, '_>, file_type: FileType, len: u64);
    fn visit_error(&mut self, dir: &Path, e: std::io::Error);
}

// TODO: move to declared
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a, 'b> {
    pub path: &'b Path,
    pub maybe_path_type: Option<declarative::PathType>,
    pub maybe_properties: Option<declarative::Properties<'a>>,
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
    maybe_node_children: Option<&'a NodeChildren>,
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
                            maybe_node_children,
                            inherited_properties,
                            visitor,
                        ) {
                            Ok(()) => (),
                            Err(e) => visitor.visit_error(&entry.path(), e),
                        };
                    }
                    Err(e) => visitor.visit_error(dir, e),
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir, e);
        }
    }

    Ok(())
}

fn process_entry<'a>(
    entry: &std::fs::DirEntry,
    maybe_node_children: Option<&'a NodeChildren>,
    inherited_properties: Option<declarative::Properties<'a>>,
    visitor: &mut impl Visitor<'a>,
) -> Result<(), std::io::Error> {
    let metadata = entry.metadata()?;
    let path = entry.path();
    let path_type = PathType::from(entry.file_type()?);
    let maybe_tree_node = maybe_node_children
        .and_then(|tree_directory| tree_directory.get(entry.file_name().as_os_str()));

    let maybe_declared_properties = maybe_tree_node
        .and_then(|tree_node| tree_node.get_properties())
        .or(inherited_properties);

    let maybe_declared_path_type = maybe_tree_node.map(|tree_node| tree_node.path_type());
    let entry = Entry {
        path: &path,
        maybe_path_type: maybe_declared_path_type,
        maybe_properties: maybe_declared_properties,
    };

    let maybe_children = maybe_tree_node.and_then(|tree_node| tree_node.get_children());

    match path_type {
        PathType::Directory => {
            if visitor
                .visit_dir(
                    entry,
                    maybe_children.is_some_and(|children| !children.is_empty()),
                )
                .is_continue()
            {
                let propagate_properties = match maybe_declared_path_type {
                    Some(declarative::PathType::Directory { owns_contents }) => owns_contents,
                    None => true,
                    _ => false,
                };

                walk_dir(
                    &path,
                    maybe_children,
                    if propagate_properties {
                        maybe_declared_properties
                    } else {
                        None
                    },
                    visitor,
                )?;
            }
        }
        PathType::File(file_type) => {
            visitor.visit_file(entry, file_type, metadata.len());
        }
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    Declarative(declarative::FileType),
    Other(fs::FileType),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
