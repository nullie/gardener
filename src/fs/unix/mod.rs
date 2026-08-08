use std::{fs, os::unix::fs::FileTypeExt};

use crate::declarative::{self};

pub mod walker;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    Declarative(declarative::FileType),
    Other(fs::FileType),
}

pub type PathType = super::Entry<(), FileType>;

impl PathType {
    pub fn from_declarative_path_type(declared_path_type: declarative::PathType) -> Self {
        match declared_path_type {
            declarative::PathType::Directory(_) => Self::Directory(()),
            declarative::PathType::File(file_type) => Self::File(FileType::Declarative(file_type)),
        }
    }

    pub fn from_std_file_type(std_file_type: std::fs::FileType) -> Self {
        if std_file_type.is_dir() {
            Self::Directory(())
        } else {
            let file_type = if std_file_type.is_file() {
                FileType::Declarative(declarative::FileType::Regular)
            } else if std_file_type.is_symlink() {
                FileType::Declarative(declarative::FileType::Symlink)
            } else if std_file_type.is_char_device() {
                FileType::Declarative(declarative::FileType::CharDevice)
            } else if std_file_type.is_block_device() {
                FileType::Declarative(declarative::FileType::BlockDevice)
            } else if std_file_type.is_fifo() {
                FileType::Declarative(declarative::FileType::Fifo)
            } else {
                FileType::Other(std_file_type)
            };

            Self::File(file_type)
        }
    }
}

#[cfg(test)]
mod tests;
