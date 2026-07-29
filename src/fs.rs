use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::declarative::tree::{ClosedNodeType, Node, Tree};
use crate::{config::OwnerModule, declarative::DeclaredFileType};

pub trait Visitor<'a> {
    fn visit_dir(
        &mut self,
        path: PathBuf,
        maybe_owner: Option<OwnerModule<'a>>,
        expected: Option<FileType>,
        expected_children: bool,
    ) -> bool;
    fn visit_file(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule<'a>>,
        file_type: FileType,
        expected: Option<FileType>,
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
                let entry = entry?;
                let path = entry.path();
                let file_type = FileType::new(entry.file_type()?);
                let maybe_tree_node = maybe_tree_directory
                    .and_then(|tree_directory| tree_directory.get(entry.file_name().as_os_str()));
                let maybe_owner = maybe_tree_node.and_then(|tree_node| match tree_node {
                    Node::Open(maybe_owner, _) => maybe_owner.to_owned(),
                    Node::Closed(owner, _) => Some(owner.to_owned()),
                });
                let maybe_expected_file_type = maybe_tree_node.map(|tree_node| match tree_node {
                    Node::Open(_, _) => FileType::Directory,
                    Node::Closed(_, ClosedNodeType::ClosedDirectory) => FileType::Directory,
                    Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                        FileType::File(*declared_file_type)
                    }
                });
                let maybe_children = match maybe_tree_node {
                    Some(Node::Open(_maybe_owner, children)) => Some(children),
                    _ => None,
                };

                match file_type {
                    FileType::Directory => {
                        if visitor.visit_dir(
                            path.clone(),
                            maybe_owner,
                            maybe_expected_file_type,
                            maybe_children.is_some(),
                        ) {
                            visit_dir(&path, maybe_children, visitor)?;
                        }
                    }
                    file_type => {
                        visitor.visit_file(path, maybe_owner, file_type, maybe_expected_file_type);
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
pub enum FileType {
    Directory,
    File(DeclaredFileType),
    Other(fs::FileType),
}

impl FileType {
    fn new(file_type: fs::FileType) -> Self {
        if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_file() {
            FileType::File(DeclaredFileType::Regular)
        } else if file_type.is_symlink() {
            FileType::File(DeclaredFileType::Symlink)
        } else {
            FileType::Other(file_type)
        }
    }
}
