use std::{collections::BTreeMap, ffi::OsString, path::Path};

use crate::declarative::{self, FileType, PathType};
use eyre::Context;
use thiserror::Error;

pub struct Tree<'a> {
    pub root: Children<'a>,
}

pub type Children<'a> = BTreeMap<OsString, Node<'a>>;

#[derive(Debug)]
pub enum Node<'a> {
    Open(Option<declarative::Properties<'a>>, Children<'a>),
    Closed(declarative::Properties<'a>, ClosedNodeType),
}

impl<'a> Node<'a> {
    fn new(path_type: declarative::PathType, properties: declarative::Properties<'a>) -> Self {
        match path_type {
            PathType::OpenDirectory => Node::Open(Some(properties), BTreeMap::new()),
            PathType::ClosedDirectory => Node::Closed(properties, ClosedNodeType::ClosedDirectory),
            PathType::File(file_type) => Node::Closed(properties, ClosedNodeType::File(file_type)),
        }
    }

    pub fn maybe_children(&'a self) -> Option<&'a Children<'a>> {
        match self {
            Node::Open(_, children) => Some(children),
            _ => None,
        }
    }

    pub fn path_type(&self) -> PathType {
        match self {
            Node::Open(_, _) => PathType::OpenDirectory,
            Node::Closed(_, ClosedNodeType::ClosedDirectory) => PathType::ClosedDirectory,
            Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                PathType::File(*declared_file_type)
            }
        }
    }

    pub fn maybe_properties(&self) -> Option<declarative::Properties<'a>> {
        let maybe_properties = match self {
            Node::Open(maybe_properties, _) => maybe_properties.as_ref(),
            Node::Closed(properties, _) => Some(properties),
        };

        maybe_properties.map(|properties| declarative::Properties {
            owner: properties.owner,
            storage_class: properties.storage_class,
        })
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
        path: &Path,
        path_type: declarative::PathType,
        properties: declarative::Properties<'a>,
    ) -> eyre::Result<()> {
        self.add_path_by_components(
            Self::path_to_components(path)?.into_iter(),
            path_type,
            properties,
        )
        .wrap_err_with(|| format!("path: {path:?}"))
    }

    fn add_path_by_components(
        &mut self,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        path_type: declarative::PathType,
        properties: declarative::Properties<'a>,
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
                vacant.insert(Node::new(path_type, properties));
            }
            std::collections::btree_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();

                match (occupied, path_type) {
                    (Node::Open(maybe_properties @ None, _), PathType::OpenDirectory) => {
                        *maybe_properties = Some(properties);
                    }
                    (occupied @ Node::Open(_, _), PathType::ClosedDirectory) => {
                        // Closed directory swallows directories below
                        *occupied = Node::Closed(properties, ClosedNodeType::ClosedDirectory);
                    }
                    (occupied, path_type) => {
                        let existing_path_type = occupied.path_type();

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
