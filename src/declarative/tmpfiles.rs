use std::path::Path;

use systemd_tmpfiles::Directive;

use crate::declarative::{self, FileType, Owner, PathType};

pub fn add_systemd_tmpfiles(tree: &mut declarative::Tree) -> rootcause::Result<()> {
    let owner = Owner::AdhocSystem {
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
        let maybe_path_type = match entry.directive() {
            Directive::CreateSymlink { .. } => Some(PathType::File(FileType::Symlink)),
            Directive::CreateFile { .. } | Directive::WriteToFile { .. } => {
                Some(PathType::File(FileType::Regular))
            }
            Directive::CreateFifo { .. } => Some(PathType::File(FileType::Fifo)),
            Directive::CreateCharDeviceNode { .. } => Some(PathType::File(FileType::CharDevice)),
            Directive::CreateBlockDeviceNode { .. } => Some(PathType::File(FileType::BlockDevice)),
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                Some(PathType::Directory(declarative::DirectoryProperties {
                    owns_contents: false,
                }))
            }
            _ => None,
        };

        if let Some(path_type) = maybe_path_type {
            // FIXME: return error
            assert!(!entry.path_is_glob());

            let path = Path::new(entry.path());
            let properties = declarative::Properties {
                owner,
                storage_class: declarative::StorageClass::Ephemeral,
            };

            tree.add_path(path, path_type, properties)
                .or_else(|tree_error| match tree_error {
                    declarative::tree::TreeError::ExistingProperties(_existing_properties) => {
                        // systemd-tmpfiles are added after declared ones and should not
                        // override them
                        Ok(())
                    }
                    e => Err(rootcause::report!(e.to_string()).attach(format!("path: {path:?}"))),
                })?;
        };
    }

    Ok(())
}
