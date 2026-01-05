use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    io::BufReader,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

use eyre::Context;
use serde::{Deserialize, Deserializer};
use systemd_tmpfiles::Directive;
use thiserror::Error;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    modules: HashMap<String, Module>,
    #[serde(default)]
    users: HashMap<String, DataConfig>,
}

#[derive(Deserialize, Debug, Default)]
struct Module {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    user: DataConfig,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    system: DataConfig,
}

#[derive(Deserialize, Debug, Default)]
struct DataConfig {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    cache: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    data: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    ephemeral: Paths,
}

impl DataConfig {
    fn add_to_tree<'a>(
        &self,
        owner: Owner<'a>,
        root: &Path,
        tree: &mut Tree<'a>,
    ) -> eyre::Result<()> {
        self.cache.add_to_tree(owner, root, tree)?;
        self.data.add_to_tree(owner, root, tree)?;
        self.ephemeral.add_to_tree(owner, root, tree)?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Default)]
struct Paths {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    intermediate: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    directories: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    files: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    symlinks: Vec<PathBuf>,
}

impl Paths {
    fn add_to_tree<'a>(
        &self,
        owner: Owner<'a>,
        root: &Path,
        tree: &mut Tree<'a>,
    ) -> eyre::Result<()> {
        for directory in &self.intermediate {
            tree.add_directory_path(owner, &root.join(directory))?;
        }
        for directory in &self.directories {
            tree.add_entry_path(owner, &root.join(directory), Entry::Directory)?;
        }
        for file in &self.files {
            tree.add_entry_path(owner, &root.join(file), Entry::File)?;
        }
        for symlink in &self.symlinks {
            tree.add_entry_path(owner, &root.join(symlink), Entry::Symlink)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum Owner<'a> {
    User(&'a str),
    Module(&'a str),
}

fn main() -> eyre::Result<()> {
    let input = std::fs::File::open("/etc/gardener.json")?;
    let buffered = BufReader::new(input);

    let config: Config = serde_json::from_reader(buffered)?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    for (user, data_config) in &config.users {
        let home_dir = Path::new("/home").join(user);

        data_config.add_to_tree(Owner::User(user), &home_dir, &mut tree)?;

        for (name, module) in &config.modules {
            module
                .user
                .add_to_tree(Owner::Module(name), &home_dir, &mut tree)?;
        }
    }

    let root = Path::new("/");

    for (name, module) in &config.modules {
        module
            .system
            .add_to_tree(Owner::Module(name), root, &mut tree)?;
    }

    let mut unknown_dirs = Vec::new();
    let mut unknown_files = Vec::new();

    visit_dirs(
        Path::new("/"),
        &tree.root,
        &mut unknown_dirs,
        &mut unknown_files,
    )?;

    println!("Unknown dirs:");
    for dir in unknown_dirs {
        println!("{dir:?}");
    }

    println!("Unknown files:");
    for file in unknown_files {
        println!("{file:?}");
    }

    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
enum Entry {
    File,
    Symlink,
    Directory,
}

#[derive(Debug)]
enum TreeNode<'a> {
    Entry(Owner<'a>, Entry),
    Directory(HashMap<OsString, TreeNode<'a>>),
}

struct Tree<'a> {
    root: HashMap<OsString, TreeNode<'a>>,
}

impl<'a> Tree<'a> {
    fn new() -> Self {
        Self {
            root: HashMap::new(),
        }
    }

    fn path_to_components(path: &Path) -> Result<Vec<OsString>, TreeError> {
        let mut components = path.components();

        if components.next() != Some(std::path::Component::RootDir) {
            panic!("Path must be absolute");
        }

        let intermediate: Vec<OsString> = components
            .map(|c| match c {
                std::path::Component::Normal(c) => Ok(c.to_owned()),
                c => Err(TreeError::UnexpectedPathComponent(format!("{:?}", c))),
            })
            .collect::<Result<_, TreeError>>()?;

        Ok(intermediate)
    }

    fn add_directory_path(&mut self, owner: Owner, path: &Path) -> eyre::Result<()> {
        self.add_directory(owner, Self::path_to_components(path)?)
            .wrap_err_with(|| format!("path: {path:?}"))
    }

    fn add_entry_path(&mut self, owner: Owner<'a>, path: &Path, entry: Entry) -> eyre::Result<()> {
        self.add_entry(owner, Self::path_to_components(path)?.into_iter(), entry)
            .wrap_err_with(|| format!("path: {path:?}"))
    }

    fn add_directory(
        &mut self,
        owner: Owner,
        components: impl IntoIterator<Item = OsString>,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        for component in components {
            let entry = directory
                .entry(component)
                .or_insert_with(|| TreeNode::Directory(HashMap::new()));

            match entry {
                TreeNode::Directory(d) => {
                    directory = d;
                }
                _ => return Err(TreeError::OverlappingPath),
            }
        }

        Ok(())
    }

    fn add_entry(
        &mut self,
        owner: Owner<'a>,
        mut components: impl DoubleEndedIterator<Item = OsString>,
        entry: Entry,
    ) -> Result<(), TreeError> {
        let mut directory = &mut self.root;

        let Some(last_component) = components.next_back() else {
            return Err(TreeError::EmptyPath);
        };

        for component in components {
            let entry = directory
                .entry(component)
                .or_insert_with(|| TreeNode::Directory(HashMap::new()));

            match entry {
                TreeNode::Directory(d) => {
                    directory = d;
                }
                TreeNode::Entry(_, Entry::Directory) => return Ok(()),
                _ => return Err(TreeError::OverlappingPath),
            }
        }

        match directory.entry(last_component) {
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(TreeNode::Entry(owner, entry));
            }
            std::collections::hash_map::Entry::Occupied(occupied) => {
                let occupied = occupied.into_mut();
                if let TreeNode::Directory(d) = occupied
                    && (d.is_empty() || matches!(entry, Entry::Directory))
                {
                    *occupied = TreeNode::Entry(owner, entry);
                } else {
                    return Err(TreeError::OverlappingPath);
                }
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
enum TreeError {
    #[error("path is empty")]
    EmptyPath,
    #[error("path is overlapping")]
    OverlappingPath,
    #[error("unexpected component")]
    UnexpectedPathComponent(String),
}

fn visit_dirs(
    dir: &Path,
    tree_directory: &HashMap<OsString, TreeNode>,
    unknown_dirs: &mut Vec<PathBuf>,
    unknown_files: &mut Vec<PathBuf>,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_type = entry.file_type()?;
                let tree_entry = tree_directory.get(entry.file_name().as_os_str());

                match tree_entry {
                    Some(TreeNode::Directory(entry_tree_directory)) if file_type.is_dir() => {
                        visit_dirs(&path, entry_tree_directory, unknown_dirs, unknown_files)?;
                    }
                    None => {
                        if file_type.is_dir() {
                            unknown_dirs.push(path);
                        } else {
                            unknown_files.push(path);
                        };
                    }
                    Some(TreeNode::Entry(owner, expected_entry)) => {
                        let found_entry = if file_type.is_dir() {
                            Entry::Directory
                        } else if file_type.is_file()
                            || file_type.is_fifo()
                            || file_type.is_socket()
                            || file_type.is_char_device()
                            || file_type.is_block_device()
                        {
                            Entry::File
                        } else if file_type.is_symlink() {
                            Entry::Symlink
                        } else {
                            panic!("Unknown file type: {file_type:?}")
                        };

                        if &found_entry != expected_entry {
                            eprintln!(
                                "{path:?} expected {expected_entry:?}, found {found_entry:?}"
                            );
                        }
                    }
                    Some(tree_entry) => {
                        let found = if file_type.is_dir() {
                            "directory"
                        } else if file_type.is_symlink() {
                            "symlink"
                        } else if file_type.is_file() {
                            "file"
                        } else if file_type.is_fifo()
                            || file_type.is_socket()
                            || file_type.is_char_device()
                            || file_type.is_block_device()
                        {
                            "special file"
                        } else {
                            "unknown file"
                        };

                        let expected = match tree_entry {
                            TreeNode::Directory(_) | TreeNode::Entry(_, Entry::Directory) => {
                                "directory"
                            }
                            TreeNode::Entry(_, Entry::Symlink) => "symlink",
                            TreeNode::Entry(_, Entry::File) => "file",
                        };

                        eprintln!("{path:?}: unexpected entry, expected {expected}, found {found}");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read directory {:?}: {}", dir, e)
        }
    }

    Ok(())
}

fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
    let owner = Owner::Module("systemd-tmpfiles");
    let output = std::process::Command::new("systemd-tmpfiles")
        .arg("--cat-config")
        .output()?;

    // FIXME: return error
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout)?;

    let parsed = systemd_tmpfiles::parser::parse_str(&output)?;

    for entry in parsed {
        match entry.directive() {
            Directive::CreateSymlink { .. } => {
                tree.add_entry_path(owner, Path::new(entry.path()), Entry::Symlink)?;
                assert!(!entry.path_is_glob());
            }
            Directive::CreateFile { .. }
            | Directive::CreateFifo { .. }
            | Directive::CreateCharDeviceNode { .. }
            | Directive::CreateBlockDeviceNode { .. }
            | Directive::WriteToFile { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                tree.add_entry_path(owner, Path::new(entry.path()), Entry::File)?;
            }
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                tree.add_directory_path(owner, Path::new(entry.path()))?;
            }
            _ => (),
        }
    }

    Ok(())
}
