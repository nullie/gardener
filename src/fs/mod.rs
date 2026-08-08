pub mod unix;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry<D, F> {
    Directory(D),
    File(F),
}

impl<D, F> Entry<D, F> {
    pub fn map_dir<T>(self, f: impl FnOnce(D) -> T) -> Entry<T, F> {
        match self {
            Self::Directory(d) => Entry::Directory(f(d)),
            Self::File(f) => Entry::File(f),
        }
    }

    pub fn map_dir_or<T>(self, default: T, f: impl FnOnce(D) -> T) -> T {
        match self {
            Self::Directory(d) => f(d),
            Self::File(_) => default,
        }
    }

    pub fn map_file<T>(self, f: impl FnOnce(F) -> T) -> Entry<D, T> {
        match self {
            Self::Directory(d) => Entry::Directory(d),
            Self::File(file_entry) => Entry::File(f(file_entry)),
        }
    }

    pub fn as_ref(&self) -> Entry<&D, &F> {
        match self {
            Self::Directory(d) => Entry::Directory(d),
            Self::File(f) => Entry::File(f),
        }
    }

    pub fn dir(self) -> Option<D> {
        match self {
            Self::Directory(d) => Some(d),
            Self::File(_) => None,
        }
    }

    pub fn file(self) -> Option<F> {
        match self {
            Self::Directory(_) => None,
            Self::File(f) => Some(f),
        }
    }
}

impl<T> Entry<T, T> {
    pub fn unwrap_either(self) -> T {
        match self {
            Self::Directory(d) => d,
            Self::File(f) => f,
        }
    }
}
