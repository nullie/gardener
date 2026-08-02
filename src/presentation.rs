use std::path::PathBuf;

use crate::{declarative, fs};

pub struct UntrackedPath {
    pub path: PathBuf,
    pub path_type: fs::PathType,
}

impl UntrackedPath {
    pub fn new(path: PathBuf, path_type: fs::PathType) -> Self {
        Self { path, path_type }
    }
}

impl std::fmt::Display for UntrackedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_symbol = match &self.path_type {
            fs::PathType::Directory => 'd',
            fs::PathType::File(file_type) => match file_type {
                fs::FileType::Declarative(declared_type) => match declared_type {
                    declarative::FileType::Regular => 'f',
                    declarative::FileType::Symlink => 's',
                    declarative::FileType::Fifo => 'p',
                    declarative::FileType::CharDevice => 'b',
                    declarative::FileType::BlockDevice => 'l',
                },

                fs::FileType::Other(_file_type) => '?',
            },
        };

        write!(f, "{} {}", type_symbol, self.path.display())
    }
}
