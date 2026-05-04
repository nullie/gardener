use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::declarative::DeclaredFileType;
use crate::{config::OwnerModule, declarative::DeclaredPathType};
use eyre::Context;
use thiserror::Error;

pub struct Tree<'a> {
    pub root: Children<'a>,
}

pub type Children<'a> = BTreeMap<OsString, Node<'a>>;

#[derive(Debug)]
pub enum Node<'a> {
    Open(Option<OwnerModule<'a>>, Children<'a>),
    Closed(OwnerModule<'a>, ClosedNodeType),
}

impl<'a> Node<'a> {
    fn to_declared_path_type(&self) -> DeclaredPathType {
        match self {
            Node::Open(_, _) => DeclaredPathType::OpenDirectory,
            Node::Closed(_, ClosedNodeType::ClosedDirectory) => DeclaredPathType::ClosedDirectory,
            Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                DeclaredPathType::File(*declared_file_type)
            }
        }
    }
}

#[derive(Debug)]
pub enum ClosedNodeType {
    ClosedDirectory,
    File(DeclaredFileType),
}

impl<'a> Tree<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            root: BTreeMap::new(),
        }
    }

    fn path_to_components(path: &Path) -> Result<Vec<OsString>, TreeError> {
        let mut components = path.components();

        if components.next() != Some(std::path::Component::RootDir) {
            panic!("Path must be absolute");
        }

        let intermediate: Vec<OsString> = components
            .map(|c| match c {
                std::path::Component::Normal(c) => Ok(c.to_owned()),
                c => Err(TreeError::UnexpectedPathComponent(format!("{:?}", c))),
            })
            .collect::<Result<_, TreeError>>()?;

        Ok(intermediate)
    }

    pub fn add_path(
        &mut self,
        owner: OwnerModule<'a>,
        path: &Path,
        path_type: DeclaredPathType,
    ) -> eyre::Result<()> {
        self.add_path_by_components(
            owner,
            Self::path_to_components(path)?.into_iter(),
            path_type,
        )
        .wrap_err_with(|| format!("path: {path:?}"))
    }

    fn add_path_by_components(
        &mut self,
        owner: OwnerModule<'a>,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        path_type: DeclaredPathType,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in components {
            let entry = directory
                .entry(dbg!(component))
                .or_insert_with(|| Node::Open(None, BTreeMap::new()));

            match entry {
                Node::Open(_, d) => {
                    directory = d;
                }
                Node::Closed(..) => {
                    // Swallow directories below closed
                    return Ok(());
                }
            }
        }

        match directory.entry(last_component) {
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(match path_type {
                    DeclaredPathType::OpenDirectory => Node::Open(Some(owner), BTreeMap::new()),
                    DeclaredPathType::ClosedDirectory => {
                        Node::Closed(owner, ClosedNodeType::ClosedDirectory)
                    }
                    DeclaredPathType::File(file_type) => {
                        Node::Closed(owner, ClosedNodeType::File(file_type))
                    }
                });
            }
            std::collections::btree_map::Entry::Occupied(occupied) => {
                let occupied = dbg!(occupied).into_mut();

                match (occupied, path_type) {
                    (Node::Open(maybe_owner @ None, _), DeclaredPathType::OpenDirectory) => {
                        *maybe_owner = Some(owner);
                    }
                    (occupied @ Node::Open(_, _), DeclaredPathType::ClosedDirectory) => {
                        // Closed directory swallows directories below
                        *occupied = Node::Closed(owner, ClosedNodeType::ClosedDirectory);
                    }
                    (occupied, path_type) => {
                        let existing_path_type = occupied.to_declared_path_type();

                        if existing_path_type != path_type {
                            // TODO: conflicting path
                            return Err(TreeError::OverlappingPath);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
enum TreeError {
    #[error("path is empty")]
    EmptyPath,
    #[error("path is overlapping")]
    OverlappingPath,
    #[error("unexpected component")]
    UnexpectedPathComponent(String),
}

impl<'a> IntoIterator for Tree<'a> {
    type Item = (PathBuf, DeclaredPathType);

    type IntoIter = TreeIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIterator::new(self)
    }
}

pub struct TreeIterator<'a> {
    path: PathBuf,
    stack: Vec<std::collections::btree_map::IntoIter<OsString, Node<'a>>>,
}

impl<'a> TreeIterator<'a> {
    fn new(tree: Tree<'a>) -> Self {
        Self {
            path: PathBuf::from("/"),
            stack: vec![tree.root.into_iter()],
        }
    }
}

impl<'a> Iterator for TreeIterator<'a> {
    type Item = (PathBuf, DeclaredPathType);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            if let Some((name, item)) = top.next() {
                match item {
                    Node::Closed(_owner, closed_node_type) => {
                        let declared_path_type = match closed_node_type {
                            ClosedNodeType::ClosedDirectory => DeclaredPathType::ClosedDirectory,
                            ClosedNodeType::File(declared_file_type) => {
                                DeclaredPathType::File(declared_file_type)
                            }
                        };
                        return Some((self.path.join(name), declared_path_type));
                    }
                    Node::Open(maybe_owner, children) => {
                        self.path.push(&name);
                        self.stack.push(children.into_iter());

                        if let Some(_owner) = maybe_owner {
                            return Some((self.path.join(name), DeclaredPathType::OpenDirectory));
                        }
                    }
                }
            } else {
                self.stack.pop().unwrap();
                if !self.stack.is_empty() {
                    assert!(self.path.pop());
                }
            }
        }

        None
    }
}
