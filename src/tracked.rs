use std::{borrow::Cow, io};

use crate::{
    config::Config,
    declarative::{DeclaredFileType, DeclaredPathType},
};

pub fn check_tracked() -> eyre::Result<()> {
    let config = Config::load()?;

    for (path, properties) in config.paths() {
        if !properties.owner_module.enabled() {
            continue;
        }

        let err_message = match path.symlink_metadata() {
            Ok(metadata) => (match properties.path_type {
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
            Err(err) => Some(match err.kind() {
                io::ErrorKind::NotFound => "not found".into(),
                _ => format!("error: {}", err).into(),
            }),
        };

        if let Some(err_message) = err_message {
            println!("{}: {}", path.display(), err_message);
        }
    }

    Ok(())
}
