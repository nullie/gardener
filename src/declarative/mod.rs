pub mod tmpfiles;
pub mod tree;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DeclaredPathType {
    OpenDirectory,
    ClosedDirectory,
    File(DeclaredFileType),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DeclaredFileType {
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

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct DeclaredPathProperties<'a> {
    pub path_type: DeclaredPathType,
    pub owner_module: OwnerModule<'a>,
    pub storage_class: StorageClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OwnerModule<'a> {
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

impl<'a> OwnerModule<'a> {
    pub fn enabled(&self) -> bool {
        match self {
            OwnerModule::AdhocSystem { .. } => true,
            OwnerModule::AdhocUser { .. } => true,
            OwnerModule::System { enabled, .. } => *enabled,
            OwnerModule::User { enabled, .. } => *enabled,
        }
    }

    pub fn should_backup(&self) -> bool {
        false
    }
}
