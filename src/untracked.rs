use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

use systemd_tmpfiles::Directive;

use crate::config::{Config, EntryType, Owner};
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
                let file_type = entry.file_type()?;
                let tree_entry = tree_directory.get(entry.file_name().as_os_str());

                match tree_entry {
                    Some(TreeNode::Directory(entry_tree_directory)) if file_type.is_dir() => {
                        visit_dirs(&path, entry_tree_directory, unknown_dirs, unknown_files)?;
                    }
                    None => {
                        if file_type.is_dir() {
                            unknown_dirs.push(path);
                        } else {
                            unknown_files.push(path);
                        };
                    }
                    Some(TreeNode::Entry(owner, expected_entry)) => {
                        let found_entry = if file_type.is_dir() {
                            EntryType::Directory
                        } else if file_type.is_file()
                            || file_type.is_fifo()
                            || file_type.is_socket()
                            || file_type.is_char_device()
                            || file_type.is_block_device()
                        {
                            EntryType::File
                        } else if file_type.is_symlink() {
                            EntryType::Symlink
                        } else {
                            panic!("Unknown file type: {file_type:?}")
                        };

                        if &found_entry != expected_entry {
                            eprintln!(
                                "{path:?} expected {expected_entry:?}, found {found_entry:?}"
                            );
                        }
                    }
                    Some(tree_entry) => {
                        let found = if file_type.is_dir() {
                            "directory"
                        } else if file_type.is_symlink() {
                            "symlink"
                        } else if file_type.is_file() {
                            "file"
                        } else if file_type.is_fifo()
                            || file_type.is_socket()
                            || file_type.is_char_device()
                            || file_type.is_block_device()
                        {
                            "special file"
                        } else {
                            "unknown file"
                        };

                        let expected = match tree_entry {
                            TreeNode::Directory(_) | TreeNode::Entry(_, EntryType::Directory) => {
                                "directory"
                            }
                            TreeNode::Entry(_, EntryType::Symlink) => "symlink",
                            TreeNode::Entry(_, EntryType::File) => "file",
                        };

                        eprintln!("{path:?}: unexpected entry, expected {expected}, found {found}");
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

fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
    let owner = Owner::Module("systemd-tmpfiles");
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
