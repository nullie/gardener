use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
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
        maybe_expected_path_type: Option<crate::fs::PathType>,
        maybe_properties: Option<declarative::Properties<'a>>,
        has_declared_children: bool,
    ) -> bool;
    fn visit_file(
        &mut self,
        path: PathBuf,
        file_type: PathType,
        len: u64,
        maybe_expected_path_type: Option<crate::fs::PathType>,
        maybe_properties: Option<declarative::Properties<'a>>,
    );
    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error);
}

pub fn visit_dirs<'a>(
    dir: &Path,
    tree: &'a Tree<'a>,
    visitor: &mut impl Visitor<'a>,
) -> eyre::Result<()> {
    visit_dir(dir, Some(&tree.root), visitor)
}

fn visit_dir<'a>(
    dir: &Path,
    maybe_tree_directory: Option<&'a BTreeMap<OsString, Node>>,
    visitor: &mut impl Visitor<'a>,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.unwrap();
                let metadata = entry.metadata().unwrap();
                let path = entry.path();
                let file_type = PathType::new(entry.file_type().unwrap());
                let maybe_tree_node = maybe_tree_directory
                    .and_then(|tree_directory| tree_directory.get(entry.file_name().as_os_str()));
                let maybe_properties =
                    maybe_tree_node.and_then(|tree_node| tree_node.maybe_properties());
                let maybe_children =
                    maybe_tree_node.and_then(|tree_node| tree_node.maybe_children());
                let maybe_expected_path_type = maybe_tree_node
                    .map(|tree_node| PathType::from_declarative(tree_node.path_type()));

                match file_type {
                    PathType::Directory => {
                        if visitor.visit_dir(
                            path.clone(),
                            maybe_expected_path_type,
                            maybe_properties,
                            maybe_children.is_some(),
                        ) {
                            visit_dir(&path, maybe_children, visitor)?;
                        }
                    }
                    file_type => {
                        visitor.visit_file(
                            path,
                            file_type,
                            metadata.len(),
                            maybe_expected_path_type,
                            maybe_properties,
                        );
                    }
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir.to_owned(), e);
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PathType {
    Directory,
    File(declarative::FileType),
    Other(fs::FileType),
}

impl PathType {
    fn new(file_type: fs::FileType) -> Self {
        if file_type.is_dir() {
            PathType::Directory
        } else if file_type.is_file() {
            PathType::File(declarative::FileType::Regular)
        } else if file_type.is_symlink() {
            PathType::File(declarative::FileType::Symlink)
        } else {
            PathType::Other(file_type)
        }
    }

    pub fn from_declarative(path_type: declarative::PathType) -> Self {
        match path_type {
            declarative::PathType::OpenDirectory | declarative::PathType::ClosedDirectory => {
                Self::Directory
            }
            declarative::PathType::File(file_type) => Self::File(file_type),
        }
    }
}
