pub mod unix;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry<D, F> {
    Directory(D),
    File(F),
}
