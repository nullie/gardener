use std::path::Path;

use systemd_tmpfiles::Directive;

use crate::declarative::{self, FileType, Owner, PathType, tree::Tree};

pub fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
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
                Some(PathType::OpenDirectory)
            }
            _ => None,
        };

        if let Some(path_type) = maybe_path_type {
            // FIXME: return error
            assert!(!entry.path_is_glob());

            tree.add_path(
                Path::new(entry.path()),
                declarative::Properties {
                    owner,
                    path_type,
                    storage_class: declarative::StorageClass::Ephemeral,
                },
            )?;
        }
    }

    Ok(())
}
