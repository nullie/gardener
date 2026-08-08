use std::{fs, ops::ControlFlow, path::Path};

use crate::decl::{
    self,
    tree::{NodeChildren, Tree},
};

pub trait Visitor<P> {
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: decl::Entry<P>,
        has_declared_children: bool,
    ) -> ControlFlow<(), ()>;
    fn visit_file(
        &mut self,
        path: &Path,
        file_type: super::FileType,
        declared: decl::Entry<P>,
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
    inherited_props: Option<P>,
    visitor: &mut impl Visitor<P>,
) -> Result<(), std::io::Error> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry_result in entries {
                match entry_result {
                    Ok(entry) => {
                        match process_entry(&entry, maybe_node_children, inherited_props, visitor) {
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
    inherited_props: Option<P>,
    visitor: &mut impl Visitor<P>,
) -> Result<(), std::io::Error> {
    let metadata = entry.metadata()?;
    let path = entry.path();
    let path_type = crate::fs::unix::PathType::from_std_file_type(entry.file_type()?);
    let maybe_tree_node =
        maybe_node_children.and_then(|tree_dir| tree_dir.get(entry.file_name().as_os_str()));

    let maybe_declared_props = maybe_tree_node
        .and_then(|tree_node| tree_node.get_props())
        .or(inherited_props);

    let maybe_declared_path_type = maybe_tree_node.map(|tree_node| tree_node.path_type());
    let declared = decl::Entry {
        maybe_path_type: maybe_declared_path_type,
        maybe_props: maybe_declared_props,
    };

    let maybe_children = maybe_tree_node.and_then(|tree_node| tree_node.get_children());

    path_type
        .map_dir(|()| {
            if visitor
                .visit_dir(
                    &path,
                    declared,
                    maybe_children.is_some_and(|children| !children.is_empty()),
                )
                .is_continue()
            {
                let propagate_props = maybe_declared_path_type
                    .is_some_and(|path_type| path_type.map_dir_or(false, |d| d.owns_contents));

                walk_dir(
                    &path,
                    maybe_children,
                    if propagate_props {
                        maybe_declared_props
                    } else {
                        None
                    },
                    visitor,
                )?;
            }

            Ok(())
        })
        .map_file(|file_type| {
            visitor.visit_file(&path, file_type, declared, metadata.len());

            Ok(())
        })
        .unwrap_either()
}
