use std::{collections::BTreeMap, ffi::OsString, path::Path};

use crate::declarative::{FileType, Owner, PathType};
use eyre::Context;
use thiserror::Error;

pub struct Tree<'a> {
    pub root: Children<'a>,
}

pub type Children<'a> = BTreeMap<OsString, Node<'a>>;

#[derive(Debug)]
pub enum Node<'a> {
    Open(Option<Owner<'a>>, Children<'a>),
    Closed(Owner<'a>, ClosedNodeType),
}

impl<'a> Node<'a> {
    fn to_declared_path_type(&self) -> PathType {
        match self {
            Node::Open(_, _) => PathType::OpenDirectory,
            Node::Closed(_, ClosedNodeType::ClosedDirectory) => PathType::ClosedDirectory,
            Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                PathType::File(*declared_file_type)
            }
        }
    }
}

#[derive(Debug)]
pub enum ClosedNodeType {
    ClosedDirectory,
    File(FileType),
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
        owner: Owner<'a>,
        path: &Path,
        path_type: PathType,
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
        owner: Owner<'a>,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        path_type: PathType,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in components {
            let entry = directory
                .entry(component)
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
                    PathType::OpenDirectory => Node::Open(Some(owner), BTreeMap::new()),
                    PathType::ClosedDirectory => {
                        Node::Closed(owner, ClosedNodeType::ClosedDirectory)
                    }
                    PathType::File(file_type) => {
                        Node::Closed(owner, ClosedNodeType::File(file_type))
                    }
                });
            }
            std::collections::btree_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();

                match (occupied, path_type) {
                    (Node::Open(maybe_owner @ None, _), PathType::OpenDirectory) => {
                        *maybe_owner = Some(owner);
                    }
                    (occupied @ Node::Open(_, _), PathType::ClosedDirectory) => {
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
