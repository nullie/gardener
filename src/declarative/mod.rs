pub mod tmpfiles;
pub mod tree;

pub type Tree<'a> = tree::Tree<Properties<'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<P> {
    pub maybe_path_type: Option<PathType>,
    pub maybe_properties: Option<P>,
}

pub type PathType = crate::fs::Entry<DirectoryProperties, FileType>;
pub type ExpectedPathType = crate::fs::Entry<(), FileType>;

impl PathType {
    pub fn expected_path_type(self) -> ExpectedPathType {
        match self {
            Self::Directory(_) => ExpectedPathType::Directory(()),
            Self::File(file_type) => ExpectedPathType::File(file_type),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct DirectoryProperties {
    pub owns_contents: bool,
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
