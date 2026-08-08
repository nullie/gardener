mod node;

use std::{
    collections::{BTreeMap, btree_map},
    ffi::OsString,
    path::Path,
};

use thiserror::Error;

pub use self::node::{DirKind, Node, NodeChildren};
use crate::decl::{self, PathType};

pub struct Tree<P> {
    pub root: NodeChildren<P>,
}

impl<P: Copy + std::fmt::Debug> Tree<P> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            root: BTreeMap::new(),
        }
    }

    fn path_to_components<'a>(path: &Path) -> Result<Vec<OsString>, TreeError<'a, P>> {
        let mut components = path.components();

        if components.next() != Some(std::path::Component::RootDir) {
            return Err(TreeError::RelativePath);
        }

        let intermediate: Vec<OsString> = components
            .map(|c| match c {
                std::path::Component::Normal(c) => Ok(c.to_owned()),
                c => Err(TreeError::UnexpectedPathComponent(format!("{:?}", c))),
            })
            .collect::<Result<_, TreeError<P>>>()?;

        Ok(intermediate)
    }

    pub fn add_path(
        &mut self,
        path: &Path,
        path_type: decl::PathType,
        props: P,
    ) -> Result<(), TreeError<'_, P>> {
        self.add_path_by_components(
            Self::path_to_components(path)?.into_iter(),
            path_type,
            props,
        )
    }

    fn add_path_by_components(
        &mut self,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        path_type: decl::PathType,
        props: P,
    ) -> Result<(), TreeError<'_, P>> {
        let mut dir = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in &mut components {
            let entry = dir.entry(component);

            match entry {
                btree_map::Entry::Vacant(vacant_entry) => {
                    return Self::insert_into_vacant(
                        vacant_entry,
                        components,
                        last_component,
                        path_type,
                        props,
                    );
                }
                btree_map::Entry::Occupied(occupied_entry) => match occupied_entry.into_mut() {
                    Node::Dir(node::Dir { children, .. }) => {
                        dir = children;
                    }
                    Node::File { .. } => return Err(TreeError::ConflictingPathType),
                },
            }
        }

        // The cycle above should only complete by consuming all components
        assert!(components.next().is_none());

        match dir.entry(last_component) {
            btree_map::Entry::Vacant(vacant) => {
                vacant.insert(Node::new(path_type, props));

                Ok(())
            }
            btree_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();

                match (occupied, path_type) {
                    (Node::Dir(existing), PathType::Dir(new)) => {
                        Self::merge_dirs(&mut existing.kind, new.owns_contents, props)
                    }
                    (occupied, path_type) => {
                        let existing_path_type = occupied.path_type();

                        if existing_path_type == path_type {
                            todo!("handle existing props");
                        } else {
                            Err(TreeError::ConflictingPathType)
                        }
                    }
                }
            }
        }
    }

    fn insert_into_vacant(
        vacant_entry: btree_map::VacantEntry<'_, OsString, Node<P>>,
        remaining_intermediate: impl DoubleEndedIterator<Item = OsString>,
        last_component: OsString,
        path_type: decl::PathType,
        props: P,
    ) -> Result<(), TreeError<'_, P>> {
        let Node::Dir(node::Dir { children, .. }) = vacant_entry.insert(Node::intermediate())
        else {
            unreachable!();
        };

        let mut dir = children;

        for component in remaining_intermediate {
            let Node::Dir(node::Dir { children, .. }) = dir
                .entry(component)
                .insert_entry(Node::intermediate())
                .into_mut()
            else {
                unreachable!()
            };

            dir = children;
        }

        assert!(
            dir.insert(last_component, Node::new(path_type, props))
                .is_none()
        );

        Ok(())
    }

    fn merge_dirs(
        existing_kind: &mut DirKind<P>,
        owns_contents: bool,
        props: P,
    ) -> Result<(), TreeError<'_, P>> {
        if matches!(
            existing_kind,
            DirKind::OwnsContents(_) | DirKind::Empty(Some(_))
        ) {
            match existing_kind {
                DirKind::OwnsContents(existing_props) => {
                    return Err(TreeError::ExistingProps(existing_props));
                }
                DirKind::Empty(maybe_props) => match maybe_props {
                    Some(existing_props) => {
                        return Err(TreeError::ExistingProps(existing_props));
                    }
                    None => unreachable!(),
                },
            }
        }

        if owns_contents {
            *existing_kind = DirKind::OwnsContents(props);
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum TreeError<'a, P: std::fmt::Debug> {
    #[error("path is empty")]
    EmptyPath,
    #[error("path is relative")]
    RelativePath,
    #[error("conflicting props: {0:?}")]
    ExistingProps(&'a mut P),
    #[error("path is overlapping")]
    ConflictingPathType,
    #[error("unexpected component")]
    UnexpectedPathComponent(String),
}
