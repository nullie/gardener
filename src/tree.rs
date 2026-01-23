use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::config::{EntryType, Owner};
use eyre::Context;
use thiserror::Error;

#[derive(Debug)]
pub enum TreeNode<'a> {
    Entry(Owner<'a>, EntryType),
    Directory(HashMap<OsString, TreeNode<'a>>),
}

pub struct Tree<'a> {
    pub root: HashMap<OsString, TreeNode<'a>>,
}

impl<'a> Tree<'a> {
    pub fn new() -> Self {
        Self {
            root: HashMap::new(),
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

    pub fn add_directory_path(&mut self, owner: Owner, path: &Path) -> eyre::Result<()> {
        self.add_directory(owner, Self::path_to_components(path)?)
            .wrap_err_with(|| format!("path: {path:?}"))
    }

    pub fn add_entry_path(
        &mut self,
        owner: Owner<'a>,
        path: &Path,
        entry: EntryType,
    ) -> eyre::Result<()> {
        self.add_entry(owner, Self::path_to_components(path)?.into_iter(), entry)
            .wrap_err_with(|| format!("path: {path:?}"))
    }

    fn add_directory(
        &mut self,
        owner: Owner,
        components: impl IntoIterator<Item = OsString>,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        for component in components {
            let entry = directory
                .entry(component)
                .or_insert_with(|| TreeNode::Directory(HashMap::new()));

            match entry {
                TreeNode::Directory(d) => {
                    directory = d;
                }
                _ => return Err(TreeError::OverlappingPath),
            }
        }

        Ok(())
    }

    fn add_entry(
        &mut self,
        owner: Owner<'a>,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        entry: EntryType,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in components {
            let entry = directory
                .entry(component)
                .or_insert_with(|| TreeNode::Directory(HashMap::new()));

            match entry {
                TreeNode::Directory(d) => {
                    directory = d;
                }
                TreeNode::Entry(_, EntryType::Directory) => return Ok(()),
                _ => return Err(TreeError::OverlappingPath),
            }
        }

        match directory.entry(last_component) {
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(TreeNode::Entry(owner, entry));
            }
            std::collections::hash_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();
                if let TreeNode::Directory(d) = occupied
                    && (d.is_empty() || matches!(entry, EntryType::Directory))
                {
                    *occupied = TreeNode::Entry(owner, entry);
                } else {
                    return Err(TreeError::OverlappingPath);
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
    type Item = (PathBuf, EntryType);

    type IntoIter = TreeIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIterator::new(self)
    }
}

pub struct TreeIterator<'a> {
    path: PathBuf,
    stack: Vec<std::collections::hash_map::IntoIter<OsString, TreeNode<'a>>>,
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
    type Item = (PathBuf, EntryType);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            if let Some((name, item)) = top.next() {
                match item {
                    TreeNode::Entry(owner, entry_type) => {
                        return Some((self.path.join(name), entry_type));
                    }
                    TreeNode::Directory(hash_map) => {
                        self.path.push(name);
                        self.stack.push(hash_map.into_iter())
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
