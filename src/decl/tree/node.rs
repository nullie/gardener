use std::{collections::BTreeMap, ffi::OsString};

use crate::{
    decl::{self, DirProps, FileType, PathType},
    fs::{self},
};

pub type NodeChildren<P> = BTreeMap<OsString, Node<P>>;

pub type Node<P> = fs::Entry<Dir<P>, File<P>>;

pub struct Dir<P> {
    pub children: NodeChildren<P>,
    pub kind: DirKind<P>,
}

pub struct File<P> {
    pub file_type: FileType,
    pub props: P,
}

#[derive(Debug)]
pub enum DirKind<P> {
    OwnsContents(P),
    Empty(Option<P>),
}

impl<P: Copy> Node<P> {
    pub(crate) fn intermediate() -> Self {
        fs::Entry::Dir(Dir {
            children: BTreeMap::new(),
            kind: DirKind::Empty(None),
        })
    }

    pub(crate) fn new(path_type: decl::PathType, props: P) -> Self {
        path_type
            .map_dir(|d| Dir {
                children: BTreeMap::new(),
                kind: if d.owns_contents {
                    DirKind::OwnsContents(props)
                } else {
                    DirKind::Empty(Some(props))
                },
            })
            .map_file(|file_type| File { file_type, props })
    }

    pub fn get_children(&self) -> Option<&NodeChildren<P>> {
        self.as_ref().map_dir(|d| &d.children).dir()
    }

    pub fn path_type(&self) -> PathType {
        self.as_ref()
            .map_dir(|d| DirProps {
                owns_contents: match d.kind {
                    DirKind::OwnsContents(_) => true,
                    DirKind::Empty(_) => false,
                },
            })
            .map_file(|f| f.file_type)
    }

    pub fn get_props(&self) -> Option<P> {
        self.as_ref()
            .map_dir(|d| d.kind.get_props())
            .map_file(|f| Some(f.props))
            .unwrap_either()
    }
}

impl<P: Copy> DirKind<P> {
    fn get_props(&self) -> Option<P> {
        match *self {
            DirKind::Empty(maybe_props) => maybe_props,
            DirKind::OwnsContents(props) => Some(props),
        }
    }
}
