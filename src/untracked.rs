use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use systemd_tmpfiles::Directive;

use crate::declarative::{
    DeclaredPathType,
    tree::{ClosedNodeType, Node, Tree},
};
use crate::{
    config::{Config, OwnerModule},
    declarative::DeclaredFileType,
};

pub fn check_untracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor {
        unknown_dirs: Vec::new(),
        unknown_files: Vec::new(),
    };

    visit_dirs(Path::new("/"), &tree.root, &mut visitor)?;

    println!("Unknown dirs:");
    for dir in visitor.unknown_dirs {
        println!("{dir:?}");
    }

    println!("Unknown files:");
    for file in visitor.unknown_files {
        println!("{file:?}");
    }

    Ok(())
}

struct SystemReport {
    untracked: Vec<Entry>,
    tracked_by_disabled_module: BTreeMap<String, Entry>,
}

struct Entry {
    path: PathBuf,
    file_type: FileType,
}

trait Visitor {
    fn visit_untracked_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        file_type: FileType,
    );
    fn visit_mismatching_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        expected: FileType,
        found: FileType,
    );
    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error);
}

struct SimpleVisitor {
    unknown_dirs: Vec<PathBuf>,
    unknown_files: Vec<PathBuf>,
}

impl Visitor for SimpleVisitor {
    fn visit_untracked_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        file_type: FileType,
    ) {
        println!("{owner:?}: {} {file_type:?}", path.display());
    }

    fn visit_mismatching_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        expected: FileType,
        found: FileType,
    ) {
        eprintln!(
            "{owner:?} {}: unexpected entry, expected {expected:?}, found {found:?}",
            path.display()
        );
    }

    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}

fn visit_dirs(
    dir: &Path,
    tree_directory: &BTreeMap<OsString, Node>,
    visitor: &mut impl Visitor,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_type = FileType::new(entry.file_type()?);
                let maybe_tree_node = tree_directory.get(entry.file_name().as_os_str());

                match (maybe_tree_node, file_type) {
                    (Some(Node::Open(_maybe_owner, children)), FileType::Directory) => {
                        visit_dirs(&path, children, visitor)?;
                    }
                    (Some(tree_node), file_type) => {
                        let maybe_owner = match tree_node {
                            Node::Open(maybe_owner, _) => *maybe_owner,
                            Node::Closed(owner, _) => Some(*owner),
                        };

                        let expected = match tree_node {
                            Node::Open(_, _) => FileType::Directory,
                            Node::Closed(_, ClosedNodeType::ClosedDirectory) => FileType::Directory,
                            Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                                FileType::File(*declared_file_type)
                            }
                        };

                        if expected != file_type {
                            visitor.visit_mismatching_path(path, maybe_owner, expected, file_type);
                        } else if let Some(owner) = maybe_owner
                            && !owner.enabled()
                        {
                            visitor.visit_untracked_path(path, Some(owner), file_type);
                        }
                    }
                    (None, file_type) => {
                        visitor.visit_untracked_path(path, None, file_type);
                    }
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir.to_owned(), e);
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Directory,
    File(DeclaredFileType),
    Other(fs::FileType),
}

impl FileType {
    fn new(file_type: fs::FileType) -> Self {
        if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_file() {
            FileType::File(DeclaredFileType::Regular)
        } else if file_type.is_symlink() {
            FileType::File(DeclaredFileType::Symlink)
        } else {
            FileType::Other(file_type)
        }
    }
}

fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
    let owner = OwnerModule::AdhocSystem {
        name: "systemd-tmpfiles",
    };
    let output = std::process::Command::new("systemd-tmpfiles")
        .arg("--cat-config")
        .output()?;

    // FIXME: return error
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout)?;

    let parsed = systemd_tmpfiles::parser::parse_str(&output)?;

    for entry in parsed {
        let maybe_entry_type = match entry.directive() {
            Directive::CreateSymlink { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::Symlink))
            }
            Directive::CreateFile { .. } | Directive::WriteToFile { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::Regular))
            }
            Directive::CreateFifo { .. } => Some(DeclaredPathType::File(DeclaredFileType::Fifo)),
            Directive::CreateCharDeviceNode { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::CharDevice))
            }
            Directive::CreateBlockDeviceNode { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::BlockDevice))
            }
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                Some(DeclaredPathType::OpenDirectory)
            }
            _ => None,
        };

        if let Some(entry_type) = maybe_entry_type {
            dbg!(entry.path(), &entry_type);
            //
            // FIXME: return error
            assert!(!entry.path_is_glob());

            tree.add_path(owner, Path::new(entry.path()), entry_type)?;
        }
    }

    Ok(())
}
