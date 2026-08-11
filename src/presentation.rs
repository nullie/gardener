use std::path::Path;

use crate::fs::{TypeChar, unix::walker};

pub struct UntrackedPath<P: AsRef<Path>> {
    pub path: P,
    pub path_type: walker::PathType,
}

impl<P: AsRef<Path>> UntrackedPath<P> {
    pub fn new(path: P, path_type: walker::PathType) -> Self {
        Self { path, path_type }
    }
}

impl<P: AsRef<Path>> std::fmt::Display for UntrackedPath<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.path_type.type_char(),
            self.path.as_ref().display()
        )
    }
}
