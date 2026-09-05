#[test]
fn borg_exclude() {
    let mut test_tree = crate::decl::tree::util::TestTree::new();

    use Backup::{Exclude, Include};

    test_tree.dir("/backup", true, Include);
    test_tree.dir("/backup/backup", true, Include);
    test_tree.dir("/backup/exclude", true, Exclude);
    test_tree.dir("/exclude", true, Exclude);
    test_tree.dir("/exclude/backup", true, Include);
    test_tree.dir("/exclude/exclude", true, Exclude);
    test_tree.dir("/exclude-non-owning", false, Exclude);
    test_tree.dir("/exclude-non-owning/backup", true, Include);
    test_tree.dir("/exclude-non-owning/exclude", true, Exclude);
    test_tree.dir("/backup-non-owning", false, Include);
    test_tree.dir("/backup-non-owning/backup", true, Include);
    test_tree.dir("/backup-non-owning/exclude", true, Exclude);
}

#[derive(Clone, Copy, Debug)]
enum Backup {
    Include,
    Exclude,
}
