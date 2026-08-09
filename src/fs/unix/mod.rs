use std::{fs, os::unix::fs::FileTypeExt};

use crate::{
    decl::{self},
    fs::TypeChar,
};

pub mod walker;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    Declarative(decl::FileType),
    Other(fs::FileType),
}

pub type PathType = super::Entry<(), FileType>;

impl PathType {
    pub fn from_declarative_path_type(declared_path_type: decl::PathType) -> Self {
        declared_path_type
            .map_dir(|_| ())
            .map_file(FileType::Declarative)
    }

    pub fn from_std_file_type(std_file_type: std::fs::FileType) -> Self {
        if std_file_type.is_dir() {
            Self::Dir(())
        } else {
            let file_type = if std_file_type.is_file() {
                FileType::Declarative(decl::FileType::Regular)
            } else if std_file_type.is_symlink() {
                FileType::Declarative(decl::FileType::Symlink)
            } else if std_file_type.is_char_device() {
                FileType::Declarative(decl::FileType::CharDevice)
            } else if std_file_type.is_block_device() {
                FileType::Declarative(decl::FileType::BlockDevice)
            } else if std_file_type.is_fifo() {
                FileType::Declarative(decl::FileType::Fifo)
            } else {
                FileType::Other(std_file_type)
            };

            Self::File(file_type)
        }
    }
}

impl TypeChar for FileType {
    fn type_char(&self) -> char {
        match self {
            Self::Declarative(declared_type) => declared_type.type_char(),
            Self::Other(_file_type) => '?',
        }
    }
}

#[cfg(test)]
mod tests;
