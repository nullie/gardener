use crate::fs::TypeChar;

pub mod walker;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Symlink,
    Socket,
    Fifo,
    CharDevice,
    BlockDevice,
}

impl TypeChar for FileType {
    fn type_char(&self) -> char {
        match self {
            Self::Regular => 'f',
            Self::Symlink => 'l',
            Self::Socket => 's',
            Self::Fifo => 'p',
            Self::CharDevice => 'c',
            Self::BlockDevice => 'b',
        }
    }
}

#[cfg(test)]
mod tests;
