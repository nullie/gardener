use std::{borrow::Cow, io};

use crate::{
    config::Config,
    decl::{FileType, PathType},
};

pub fn check_tracked() -> rootcause::Result<()> {
    let config = Config::load()?;

    for (path, path_type, props) in config.paths() {
        if !props.owner.enabled() {
            continue;
        }

        let err_message = match path.symlink_metadata() {
            Ok(metadata) => (match path_type {
                PathType::Dir { .. } => (!metadata.is_dir()).then_some("not a dir"),
                PathType::File(FileType::Regular) => (!metadata.is_file()).then_some("not a file"),
                PathType::File(FileType::Symlink) => {
                    (!metadata.is_symlink()).then_some("not a symlink")
                }
                PathType::File(_) => {
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
