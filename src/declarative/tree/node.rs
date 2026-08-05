use std::{collections::BTreeMap, ffi::OsString};

use crate::declarative::{self, FileType, PathType};

pub type NodeChildren<P> = BTreeMap<OsString, Node<P>>;

#[derive(Debug)]
pub enum Node<P> {
    Directory {
        children: NodeChildren<P>,
        kind: NodeKind<P>,
    },
    File {
        file_type: FileType,
        properties: P,
    },
}

#[derive(Debug)]
pub enum NodeKind<P> {
    OwnsContents(P),
    Empty(Option<P>),
}

impl<P: Copy> Node<P> {
    pub(crate) fn intermediate() -> Self {
        Self::Directory {
            children: BTreeMap::new(),
            kind: NodeKind::Empty(None),
        }
    }

    pub(crate) fn new(path_type: declarative::PathType, properties: P) -> Self {
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

    pub fn get_children(&self) -> Option<&NodeChildren<P>> {
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

    pub fn get_properties(&self) -> Option<P> {
        match self {
            Node::Directory { kind, .. } => kind.get_properties(),
            Node::File { properties, .. } => Some(*properties),
        }
    }
}

impl<P: Copy> NodeKind<P> {
    fn get_properties(&self) -> Option<P> {
        match *self {
            NodeKind::Empty(maybe_properties) => maybe_properties,
            NodeKind::OwnsContents(properties) => Some(properties),
        }
    }
}
