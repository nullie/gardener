use std::path::Path;

use crate::{decl, fs};

pub struct UntrackedPath<P: AsRef<Path>> {
    pub path: P,
    pub path_type: fs::unix::PathType,
}

impl<P: AsRef<Path>> UntrackedPath<P> {
    pub fn new(path: P, path_type: fs::unix::PathType) -> Self {
        Self { path, path_type }
    }
}

impl<P: AsRef<Path>> std::fmt::Display for UntrackedPath<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_symbol = match &self.path_type {
            fs::unix::PathType::Dir(()) => 'd',
            fs::unix::PathType::File(file_type) => match file_type {
                fs::unix::FileType::Declarative(declared_type) => match declared_type {
                    decl::FileType::Regular => 'f',
                    decl::FileType::Symlink => 's',
                    decl::FileType::Fifo => 'p',
                    decl::FileType::CharDevice => 'b',
                    decl::FileType::BlockDevice => 'l',
                },

                fs::unix::FileType::Other(_file_type) => '?',
            },
        };

        write!(f, "{} {}", type_symbol, self.path.as_ref().display())
    }
}
