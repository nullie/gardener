use std::{collections::BTreeMap, ffi::OsString};

use crate::{
    declarative::{self, DirectoryProperties, FileType, PathType},
    fs::{self},
};

pub type NodeChildren<P> = BTreeMap<OsString, Node<P>>;

pub type Node<P> = fs::Entry<Directory<P>, File<P>>;

pub struct Directory<P> {
    pub children: NodeChildren<P>,
    pub kind: DirKind<P>,
}

pub struct File<P> {
    pub file_type: FileType,
    pub properties: P,
}

#[derive(Debug)]
pub enum DirKind<P> {
    OwnsContents(P),
    Empty(Option<P>),
}

impl<P: Copy> Node<P> {
    pub(crate) fn intermediate() -> Self {
        fs::Entry::Directory(Directory {
            children: BTreeMap::new(),
            kind: DirKind::Empty(None),
        })
    }

    pub(crate) fn new(path_type: declarative::PathType, properties: P) -> Self {
        path_type
            .map_dir(|d| Directory {
                children: BTreeMap::new(),
                kind: if d.owns_contents {
                    DirKind::OwnsContents(properties)
                } else {
                    DirKind::Empty(Some(properties))
                },
            })
            .map_file(|file_type| File {
                file_type,
                properties,
            })
    }

    pub fn get_children(&self) -> Option<&NodeChildren<P>> {
        self.as_ref().map_dir(|d| &d.children).dir()
    }

    pub fn path_type(&self) -> PathType {
        self.as_ref()
            .map_dir(|d| DirectoryProperties {
                owns_contents: match d.kind {
                    DirKind::OwnsContents(_) => true,
                    DirKind::Empty(_) => false,
                },
            })
            .map_file(|f| f.file_type)
    }

    pub fn get_properties(&self) -> Option<P> {
        self.as_ref()
            .map_dir(|d| d.kind.get_properties())
            .map_file(|f| Some(f.properties))
            .unwrap_either()
    }
}

impl<P: Copy> DirKind<P> {
    fn get_properties(&self) -> Option<P> {
        match *self {
            DirKind::Empty(maybe_properties) => maybe_properties,
            DirKind::OwnsContents(properties) => Some(properties),
        }
    }
}
