use crate::fs::TypeChar;

pub mod walker;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType<O> {
    Regular,
    Symlink,
    Fifo,
    CharDevice,
    BlockDevice,
    Other(O),
}

impl<O> FileType<O> {
    pub fn map_other<T>(self, f: impl FnOnce(O) -> T) -> FileType<T> {
        match self {
            FileType::Regular => FileType::Regular,
            FileType::Symlink => FileType::Symlink,
            FileType::Fifo => FileType::Fifo,
            FileType::CharDevice => FileType::CharDevice,
            FileType::BlockDevice => FileType::BlockDevice,
            FileType::Other(o) => FileType::Other(f(o)),
        }
    }
}

impl<O> TypeChar for FileType<O> {
    fn type_char(&self) -> char {
        match self {
            Self::Regular => 'f',
            Self::Symlink => 'l',
            Self::Fifo => 'p',
            Self::CharDevice => 'c',
            Self::BlockDevice => 'b',
            Self::Other(_file_type) => '?',
        }
    }
}

#[cfg(test)]
mod tests;
