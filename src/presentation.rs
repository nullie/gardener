use std::path::PathBuf;

use crate::{declarative::DeclaredFileType, fs::FileType};

pub struct UntrackedPath {
    path: PathBuf,
    file_type: FileType,
}

impl UntrackedPath {
    pub fn new(path: PathBuf, file_type: FileType) -> Self {
        Self { path, file_type }
    }
}

impl std::fmt::Display for UntrackedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_symbol = match self.file_type {
            FileType::Directory => 'd',
            FileType::File(declared_type) => match declared_type {
                DeclaredFileType::Regular => 'f',
                DeclaredFileType::Symlink => 's',
                DeclaredFileType::Fifo => 'p',
                DeclaredFileType::CharDevice => 'b',
                DeclaredFileType::BlockDevice => 'l',
            },
            FileType::Other(_file_type) => '?',
        };

        write!(f, "{} {}", type_symbol, self.path.display())
    }
}
