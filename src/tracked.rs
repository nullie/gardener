use std::{borrow::Cow, fs::Metadata, path::Path};

use time::{Duration, UtcDateTime};

use crate::{
    config::Config,
    declarative::{DeclaredFileType, DeclaredPathType, tree::Tree},
};

pub fn check_tracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    config.add_to_tree(&mut tree)?;

    for (path, entry_type) in tree {
        let err_message = match path.symlink_metadata() {
            Ok(metadata) => (match entry_type {
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
            println!("{}: {}", path.to_string_lossy(), err_message);
        }
    }

    Ok(())
}

fn check_directory(path: &Path, metadata: &Metadata) -> Option<&'static str> {
    if !metadata.is_dir() {
        return Some("not a directory");
    }

    let atime = metadata.accessed().unwrap();
    let age = UtcDateTime::now() - UtcDateTime::from(atime);

    if age > Duration::days(60) {
        println!("{}: {}", path.to_string_lossy(), age.whole_days());
    }

    None
}
