use std::{fs, ops::ControlFlow, path::Path};

use crate::declarative::{
    self,
    tree::{NodeChildren, Tree},
};

pub trait Visitor<P> {
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: declarative::Entry<P>,
        has_declared_children: bool,
    ) -> ControlFlow<(), ()>;
    fn visit_file(
        &mut self,
        path: &Path,
        file_type: super::FileType,
        declared: declarative::Entry<P>,
        len: u64,
    );
    fn visit_error(&mut self, dir: &Path, e: std::io::Error);
}

pub fn walk_tree<P: Copy + std::fmt::Debug>(
    dir: &Path,
    tree: &Tree<P>,
    visitor: &mut impl Visitor<P>,
) -> rootcause::Result<()> {
    walk_dir(dir, Some(&tree.root), None, visitor).map_err(rootcause::Report::from)
}

fn walk_dir<P: Copy + std::fmt::Debug>(
    dir: &Path,
    maybe_node_children: Option<&NodeChildren<P>>,
    inherited_properties: Option<P>,
    visitor: &mut impl Visitor<P>,
) -> Result<(), std::io::Error> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry_result in entries {
                match entry_result {
                    Ok(entry) => {
                        match process_entry(
                            &entry,
                            maybe_node_children,
                            inherited_properties,
                            visitor,
                        ) {
                            Ok(()) => (),
                            Err(e) => visitor.visit_error(&entry.path(), e),
                        };
                    }
                    Err(e) => visitor.visit_error(dir, e),
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir, e);
        }
    }

    Ok(())
}

fn process_entry<P: Copy + std::fmt::Debug>(
    entry: &std::fs::DirEntry,
    maybe_node_children: Option<&NodeChildren<P>>,
    inherited_properties: Option<P>,
    visitor: &mut impl Visitor<P>,
) -> Result<(), std::io::Error> {
    let metadata = entry.metadata()?;
    let path = entry.path();
    let path_type = crate::fs::unix::PathType::from_std_file_type(entry.file_type()?);
    let maybe_tree_node = maybe_node_children
        .and_then(|tree_directory| tree_directory.get(entry.file_name().as_os_str()));

    let maybe_declared_properties = maybe_tree_node
        .and_then(|tree_node| tree_node.get_properties())
        .or(inherited_properties);

    let maybe_declared_path_type = maybe_tree_node.map(|tree_node| tree_node.path_type());
    let declared = declarative::Entry {
        maybe_path_type: maybe_declared_path_type,
        maybe_properties: maybe_declared_properties,
    };

    let maybe_children = maybe_tree_node.and_then(|tree_node| tree_node.get_children());

    match path_type {
        crate::fs::Entry::Directory(()) => {
            if visitor
                .visit_dir(
                    &path,
                    declared,
                    maybe_children.is_some_and(|children| !children.is_empty()),
                )
                .is_continue()
            {
                let propagate_properties = match maybe_declared_path_type {
                    Some(crate::fs::Entry::Directory(declarative::DirectoryProperties {
                        owns_contents,
                    })) => owns_contents,
                    None => true,
                    _ => false,
                };

                walk_dir(
                    &path,
                    maybe_children,
                    if propagate_properties {
                        maybe_declared_properties
                    } else {
                        None
                    },
                    visitor,
                )?;
            }
        }
        crate::fs::Entry::File(file_type) => {
            visitor.visit_file(&path, file_type, declared, metadata.len());
        }
    }

    Ok(())
}
