use std::borrow::Cow;

use crate::{
    config::Config,
    declarative::{DeclaredFileType, DeclaredPathType},
};

pub fn check_tracked() -> eyre::Result<()> {
    let config = Config::load()?;

    for (owner, path, path_type) in config.paths() {
        if !owner.enabled() {
            continue;
        }

        let err_message = match path.symlink_metadata() {
            Ok(metadata) => (match path_type {
                DeclaredPathType::OpenDirectory | DeclaredPathType::ClosedDirectory => {
                    (!metadata.is_dir()).then_some("not a directory")
                }
                DeclaredPathType::File(DeclaredFileType::Regular) => {
                    (!metadata.is_file()).then_some("not a file")
                }
                DeclaredPathType::File(DeclaredFileType::Symlink) => {
                    (!metadata.is_symlink()).then_some("not a symlink")
                }
                DeclaredPathType::File(_) => {
                    todo!()
                }
            })
            .map(Cow::from),
            Err(err) => Some(Cow::from(err.to_string())),
        };

        if let Some(err_message) = err_message {
            println!("{}: {}", path.display(), err_message);
        }
    }

    Ok(())
}
