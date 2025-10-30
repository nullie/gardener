use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fs,
    io::BufReader,
    os::unix::fs::DirEntryExt,
    path::{Path, PathBuf},
};

use eyre::Context;
use serde::Deserialize;
use systemd_tmpfiles::Directive;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    persistent_modules: HashMap<String, Module>,
    #[serde(default)]
    users: HashMap<String, Paths>,
}

#[derive(Deserialize, Debug, Default)]
struct Module {
    #[serde(default)]
    user: DataConfig,
    #[serde(default)]
    system: DataConfig,
}

#[derive(Deserialize, Debug, Default)]
struct DataConfig {
    #[serde(default)]
    cache: Paths,
    #[serde(default)]
    data: Paths,
    #[serde(default)]
    auto: Paths,
}

#[derive(Deserialize, Debug, Default)]
struct Paths {
    #[serde(default)]
    directories: Vec<PathBuf>,
    #[serde(default)]
    files: Vec<PathBuf>,
    #[serde(default)]
    symlinks: Vec<PathBuf>,
}

fn main() -> eyre::Result<()> {
    let auto_entries = get_systemd_tmpfiles()?;

    let input = std::fs::File::open("/etc/gardener.json")?;
    let buffered = BufReader::new(input);

    let config: Config = serde_json::from_reader(buffered)?;

    let mut cache_directories = Vec::new();
    let mut data_directories = Vec::new();
    let mut cache_files = Vec::new();
    let mut data_files = Vec::new();

    for (user, paths) in &config.users {
        let home_dir = Path::new("/home").join(user);

        for directory in &paths.directories {
            data_directories.push(home_dir.join(directory));
        }

        for (name, module) in &config.persistent_modules {
            for directory in &module.user.cache.directories {
                let path = home_dir.join(directory);
                cache_directories.push(path);
            }

            for directory in &module.user.data.directories {
                let path = home_dir.join(directory);
                data_directories.push(path);
            }
        }
    }

    for (name, module) in &config.persistent_modules {
        for directory in &module.system.cache.directories {
            cache_directories.push(directory.clone());
        }

        for directory in &module.system.data.directories {
            data_directories.push(directory.clone());
        }

        for file in &module.system.cache.files {
            cache_files.push(file.clone());
        }

        for file in &module.system.data.files {
            data_files.push(file.clone());
        }

        for file in &module.system.auto.symlinks {
            data_files.push(file.clone());
        }

        for directory in &module.system.auto.directories {
            cache_directories.push(directory.clone());
        }

        for file in &module.system.auto.files {
            cache_files.push(file.clone());
        }
    }

    let tree = make_tree(
        cache_directories
            .iter()
            .chain(&data_directories)
            .chain(auto_entries.iter().filter_map(|e| match e {
                Entry::Directory(path) => Some(path),
                _ => None,
            }))
            .map(AsRef::as_ref),
        cache_files
            .iter()
            .chain(&data_files)
            .chain(auto_entries.iter().filter_map(|e| match e {
                Entry::File(path) => Some(path),
                _ => None,
            }))
            .map(AsRef::as_ref),
    );

    let mut unknown_dirs = Vec::new();
    let mut unknown_files = Vec::new();

    visit_dirs(Path::new("/"), &tree, &mut unknown_dirs, &mut unknown_files)?;

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

#[derive(Debug)]
struct TreeNode<'a> {
    directories: HashMap<&'a OsStr, TreeNode<'a>>,
    files: HashSet<&'a OsStr>,
    is_leaf: bool,
}

impl TreeNode<'_> {
    fn new() -> Self {
        Self {
            is_leaf: false,
            directories: HashMap::new(),
            files: HashSet::new(),
        }
    }
}

fn make_tree<'a>(
    directories: impl IntoIterator<Item = &'a Path>,
    files: impl IntoIterator<Item = &'a Path>,
) -> TreeNode<'a> {
    let mut root = TreeNode::new();

    for directory in directories {
        let mut node = &mut root;

        for component in directory.strip_prefix("/").unwrap() {
            node = node
                .directories
                .entry(component)
                .or_insert_with(TreeNode::new);
        }

        node.is_leaf = true;
    }

    for file in files {
        let mut node = &mut root;

        for component in file.parent().unwrap().strip_prefix("/").unwrap() {
            node = node
                .directories
                .entry(component)
                .or_insert_with(TreeNode::new);
        }

        node.files.insert(file.file_name().unwrap());
    }

    root
}

fn visit_dirs(
    dir: &Path,
    tree_node: &TreeNode,
    unknown_dirs: &mut Vec<PathBuf>,
    unknown_files: &mut Vec<PathBuf>,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if entry.file_type()?.is_dir() {
                    if let Some(node) = tree_node.directories.get(entry.file_name().as_os_str()) {
                        if !node.is_leaf {
                            visit_dirs(&path, node, unknown_dirs, unknown_files)?;
                        }
                    } else {
                        unknown_dirs.push(path);
                    }
                } else if !tree_node.files.contains(entry.file_name().as_os_str()) {
                    unknown_files.push(path);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read directory {:?}: {}", dir, e)
        }
    }

    Ok(())
}

fn get_systemd_tmpfiles() -> eyre::Result<Vec<Entry>> {
    let output = std::process::Command::new("systemd-tmpfiles")
        .arg("--cat-config")
        .output()?;

    // FIXME: return error
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout)?;

    let parsed = systemd_tmpfiles::parser::parse_str(&output)?;

    let mut entries = Vec::new();

    for entry in parsed {
        match entry.directive() {
            Directive::CreateFile { .. }
            | Directive::CreateFifo { .. }
            | Directive::CreateSymlink { .. }
            | Directive::CreateCharDeviceNode { .. }
            | Directive::CreateBlockDeviceNode { .. }
            | Directive::WriteToFile { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                entries.push(Entry::File(entry.path().into()));
            }
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                // FIXME: return error
                assert!(!entry.path_is_glob());
                entries.push(Entry::Directory(entry.path().into()))
            }
            _ => (),
        }
    }

    Ok(entries)
}

#[derive(Debug)]
enum Entry {
    File(PathBuf),
    Directory(PathBuf),
    RecursiveDirectory(PathBuf),
}
