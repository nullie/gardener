use std::path::PathBuf;

use crate::{declarative, fs};

pub struct UntrackedPath {
    pub path: PathBuf,
    pub file_type: fs::PathType,
}

impl UntrackedPath {
    pub fn new(path: PathBuf, file_type: fs::PathType) -> Self {
        Self { path, file_type }
    }
}

impl std::fmt::Display for UntrackedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_symbol = match self.file_type {
            fs::PathType::Directory => 'd',
            fs::PathType::File(declared_type) => match declared_type {
                declarative::FileType::Regular => 'f',
                declarative::FileType::Symlink => 's',
                declarative::FileType::Fifo => 'p',
                declarative::FileType::CharDevice => 'b',
                declarative::FileType::BlockDevice => 'l',
            },
            fs::PathType::Other(_file_type) => '?',
        };

        write!(f, "{} {}", type_symbol, self.path.display())
    }
}
