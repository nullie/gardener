use std::path::Path;

use systemd_tmpfiles::Directive;

use crate::config::OwnerModule;
use crate::declarative::DeclaredPathType;
use crate::declarative::{DeclaredFileType, tree::Tree};

pub fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
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
            // FIXME: return error
            assert!(!entry.path_is_glob());

            tree.add_path(owner, Path::new(entry.path()), entry_type)?;
        }
    }

    Ok(())
}
