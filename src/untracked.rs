use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

use systemd_tmpfiles::Directive;

use crate::config::{Config, EntryType, OwnerModule};
use crate::tree::{Tree, TreeNode};

pub fn check_untracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut unknown_dirs = Vec::new();
    let mut unknown_files = Vec::new();

    visit_dirs(
        Path::new("/"),
        &tree.root,
        &mut unknown_dirs,
        &mut unknown_files,
    )?;

    println!("Unknown dirs:");
    for dir in unknown_dirs {
        println!("{dir:?}");
    }

    println!("Unknown files:");
    for file in unknown_files {
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
    type_: EntryType,
}

fn visit_dirs(
    dir: &Path,
    tree_directory: &BTreeMap<OsString, TreeNode>,
    unknown_dirs: &mut Vec<PathBuf>,
    unknown_files: &mut Vec<PathBuf>,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_type = FileType::new(entry.file_type()?);
                let tree_entry = tree_directory.get(entry.file_name().as_os_str());

                match (tree_entry, file_type) {
                    (Some(TreeNode::Directory(entry_tree_directory)), FileType::Directory) => {
                        visit_dirs(&path, entry_tree_directory, unknown_dirs, unknown_files)?;
                    }
                    (None, FileType::Directory) => {
                        unknown_dirs.push(path);
                    }
                    (None, _) => {
                        unknown_files.push(path);
                    }
                    (Some(tree_entry), found) => {
                        let expected = match tree_entry {
                            TreeNode::Directory(_) | TreeNode::Entry(_, EntryType::Directory) => {
                                FileType::Directory
                            }
                            TreeNode::Entry(_, EntryType::Symlink) => FileType::Symlink,
                            TreeNode::Entry(_, EntryType::File) => FileType::File,
                        };

                        if expected != found {
                            eprintln!(
                                "{path:?}: unexpected entry, expected {expected:?}, found {found:?}"
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read directory {:?}: {}", dir, e)
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Directory,
    Symlink,
    File,
    EphemeralFile(fs::FileType),
}

impl FileType {
    fn new(file_type: fs::FileType) -> Self {
        if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_symlink() {
            FileType::Symlink
        } else if file_type.is_file() {
            FileType::File
        } else {
            FileType::EphemeralFile(file_type)
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
        match entry.directive() {
            Directive::CreateSymlink { .. } => {
                tree.add_entry_path(owner, Path::new(entry.path()), EntryType::Symlink)?;
                assert!(!entry.path_is_glob());
            }
            Directive::CreateFile { .. }
            | Directive::CreateFifo { .. }
            | Directive::CreateCharDeviceNode { .. }
            | Directive::CreateBlockDeviceNode { .. }
            | Directive::WriteToFile { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                tree.add_entry_path(owner, Path::new(entry.path()), EntryType::File)?;
            }
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                tree.add_directory_path(owner, Path::new(entry.path()))?;
            }
            _ => (),
        }
    }

    Ok(())
}
