pub mod tmpfiles;
pub mod tree;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PathType {
    Directory { owns_contents: bool },
    File(FileType),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum FileType {
    Regular,
    Symlink,
    Fifo,
    CharDevice,
    BlockDevice,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum StorageClass {
    Ephemeral,
    Cache,
    Data,
}

impl StorageClass {
    pub fn should_backup(&self) -> bool {
        matches!(self, Self::Data)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct Properties<'a> {
    pub owner: Owner<'a>,
    pub storage_class: StorageClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Owner<'a> {
    AdhocSystem {
        name: &'a str,
    },
    AdhocUser {
        name: &'a str,
        user: &'a str,
    },
    System {
        name: &'a str,
        enabled: bool,
    },
    User {
        name: &'a str,
        user: &'a str,
        enabled: bool,
    },
}

impl<'a> Owner<'a> {
    pub fn enabled(&self) -> bool {
        match self {
            Owner::AdhocSystem { .. } => true,
            Owner::AdhocUser { .. } => true,
            Owner::System { enabled, .. } => *enabled,
            Owner::User { enabled, .. } => *enabled,
        }
    }
}
