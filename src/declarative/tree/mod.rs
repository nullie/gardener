mod node;

use std::{
    collections::{BTreeMap, btree_map},
    ffi::OsString,
    path::Path,
};

use thiserror::Error;

pub use self::node::{Node, NodeChildren, NodeKind};
use crate::declarative::{self, PathType};

pub struct Tree<'a> {
    pub root: NodeChildren<'a>,
}

impl<'a> Tree<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            root: BTreeMap::new(),
        }
    }

    fn path_to_components<'b>(path: &Path) -> Result<Vec<OsString>, TreeError<'a, 'b>> {
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
    ) -> Result<(), TreeError<'a, '_>> {
        self.add_path_by_components(
            Self::path_to_components(path)?.into_iter(),
            path_type,
            properties,
        )
    }

    fn add_path_by_components(
        &mut self,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        path_type: declarative::PathType,
        properties: declarative::Properties<'a>,
    ) -> Result<(), TreeError<'a, '_>> {
        let mut directory = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in &mut components {
            let entry = directory.entry(component);

            match entry {
                btree_map::Entry::Vacant(vacant_entry) => {
                    return Self::insert_into_vacant(
                        vacant_entry,
                        components,
                        last_component,
                        path_type,
                        properties,
                    );
                }
                btree_map::Entry::Occupied(occupied_entry) => match occupied_entry.into_mut() {
                    Node::Directory { children, .. } => {
                        directory = children;
                    }
                    Node::File { .. } => return Err(TreeError::ConflictingPathType),
                },
            }
        }

        // The cycle above should only complete by consuming all components
        assert!(components.next().is_none());

        match directory.entry(last_component) {
            btree_map::Entry::Vacant(vacant) => {
                vacant.insert(Node::new(path_type, properties));

                Ok(())
            }
            btree_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();

                match (occupied, path_type) {
                    (Node::Directory { kind, .. }, PathType::Directory { owns_contents }) => {
                        Self::merge_dirs(kind, owns_contents, properties)
                    }
                    (occupied, path_type) => {
                        let existing_path_type = occupied.path_type();

                        if existing_path_type == path_type {
                            todo!("handle existing properties");
                        } else {
                            Err(TreeError::ConflictingPathType)
                        }
                    }
                }
            }
        }
    }

    fn insert_into_vacant<'b>(
        vacant_entry: btree_map::VacantEntry<'b, OsString, Node<'a>>,
        remaining_intermediate: impl DoubleEndedIterator<Item = OsString>,
        last_component: OsString,
        path_type: declarative::PathType,
        properties: declarative::Properties<'a>,
    ) -> Result<(), TreeError<'a, 'b>> {
        let Node::Directory { children, .. } = vacant_entry.insert(Node::intermediate()) else {
            unreachable!();
        };

        let mut directory = children;

        for component in remaining_intermediate {
            let Node::Directory { children, .. } = directory
                .entry(component)
                .insert_entry(Node::intermediate())
                .into_mut()
            else {
                unreachable!()
            };

            directory = children;
        }

        assert!(
            directory
                .insert(last_component, Node::new(path_type, properties))
                .is_none()
        );

        Ok(())
    }

    fn merge_dirs<'b>(
        existing_kind: &'b mut NodeKind<'a>,
        owns_contents: bool,
        properties: declarative::Properties<'a>,
    ) -> Result<(), TreeError<'a, 'b>> {
        if matches!(
            existing_kind,
            NodeKind::OwnsContents(_) | NodeKind::Empty(Some(_))
        ) {
            match existing_kind {
                NodeKind::OwnsContents(existing_properties) => {
                    return Err(TreeError::ExistingProperties(existing_properties));
                }
                NodeKind::Empty(maybe_properties) => match maybe_properties {
                    Some(existing_properties) => {
                        return Err(TreeError::ExistingProperties(existing_properties));
                    }
                    None => unreachable!(),
                },
            }
        }

        if owns_contents {
            *existing_kind = NodeKind::OwnsContents(properties);
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum TreeError<'a, 'b> {
    #[error("path is empty")]
    EmptyPath,
    #[error("conflicting properties: {0:?}")]
    ExistingProperties(&'b mut declarative::Properties<'a>),
    #[error("path is overlapping")]
    ConflictingPathType,
    #[error("unexpected component")]
    UnexpectedPathComponent(String),
}
