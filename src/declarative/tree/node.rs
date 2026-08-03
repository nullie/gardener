use std::{collections::BTreeMap, ffi::OsString};

use crate::declarative::{self, FileType, PathType, Properties};

pub type NodeChildren<'a> = BTreeMap<OsString, Node<'a>>;

#[derive(Debug)]
pub enum Node<'a> {
    Directory {
        children: NodeChildren<'a>,
        kind: NodeKind<'a>,
    },
    File {
        file_type: FileType,
        properties: declarative::Properties<'a>,
    },
}

#[derive(Debug)]
pub enum NodeKind<'a> {
    OwnsContents(declarative::Properties<'a>),
    Empty(Option<declarative::Properties<'a>>),
}

impl<'a> Node<'a> {
    pub(crate) fn intermediate() -> Self {
        Self::Directory {
            children: BTreeMap::new(),
            kind: NodeKind::Empty(None),
        }
    }

    pub(crate) fn new(
        path_type: declarative::PathType,
        properties: declarative::Properties<'a>,
    ) -> Self {
        match path_type {
            PathType::Directory { owns_contents } => Node::Directory {
                children: BTreeMap::new(),
                kind: if owns_contents {
                    NodeKind::OwnsContents(properties)
                } else {
                    NodeKind::Empty(Some(properties))
                },
            },
            PathType::File(file_type) => Node::File {
                file_type,
                properties,
            },
        }
    }

    pub fn get_children(&'a self) -> Option<&'a NodeChildren<'a>> {
        match self {
            Node::Directory { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn path_type(&self) -> PathType {
        match self {
            Node::Directory { kind, .. } => PathType::Directory {
                owns_contents: match kind {
                    NodeKind::OwnsContents(_) => true,
                    NodeKind::Empty(_) => false,
                },
            },
            Node::File { file_type, .. } => PathType::File(*file_type),
        }
    }

    pub fn get_properties(&self) -> Option<declarative::Properties<'a>> {
        match self {
            Node::Directory { kind, .. } => kind.get_properties(),
            Node::File { properties, .. } => Some(*properties),
        }
    }
}

impl<'a> NodeKind<'a> {
    fn get_properties(&self) -> Option<Properties<'a>> {
        match *self {
            NodeKind::Empty(maybe_properties) => maybe_properties,
            NodeKind::OwnsContents(properties) => Some(properties),
        }
    }
}
