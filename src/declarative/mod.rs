pub mod tree;

#[derive(Debug, Eq, PartialEq)]
pub enum PathType {
    File,
    Symlink,
    Directory,
    EmptyDirectory,
}

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
