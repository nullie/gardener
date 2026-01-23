use std::{
    collections::HashMap,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::tree::Tree;

use serde::{Deserialize, Deserializer};

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
pub struct Config {
    #[serde(default)]
    pub modules: HashMap<String, Module>,
    #[serde(default)]
    pub users: HashMap<String, DataConfig>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Module {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub user: DataConfig,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub system: DataConfig,
}

#[derive(Deserialize, Debug, Default)]
pub struct DataConfig {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub cache: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub data: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub ephemeral: Paths,
}

#[derive(Deserialize, Debug, Default)]
pub struct Paths {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub intermediate: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub directories: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub files: Vec<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub symlinks: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub enum Owner<'a> {
    User(&'a str),
    Module(&'a str),
}

#[derive(Debug, Eq, PartialEq)]
pub enum EntryType {
    File,
    Symlink,
    Directory,
}

impl Config {
    pub fn load() -> eyre::Result<Self> {
        let input = std::fs::File::open("/etc/gardener.json")?;
        let buffered = BufReader::new(input);

        Ok(serde_json::from_reader(buffered)?)
    }

    pub fn add_to_tree<'a>(&'a self, tree: &mut Tree<'a>) -> eyre::Result<()> {
        for (user, data_config) in &self.users {
            let home_dir = Path::new("/home").join(user);

            add_data_config_to_tree(data_config, Owner::User(user), &home_dir, tree)?;

            for (name, module) in &self.modules {
                add_data_config_to_tree(&module.user, Owner::Module(name), &home_dir, tree)?;
            }
        }

        let root = Path::new("/");

        for (name, module) in &self.modules {
            add_data_config_to_tree(&module.system, Owner::Module(name), root, tree)?;
        }

        Ok(())
    }
}

fn add_data_config_to_tree<'a>(
    data_config: &DataConfig,
    owner: Owner<'a>,
    root: &Path,
    tree: &mut Tree<'a>,
) -> eyre::Result<()> {
    add_paths_to_tree(&data_config.cache, owner, root, tree)?;
    add_paths_to_tree(&data_config.data, owner, root, tree)?;
    add_paths_to_tree(&data_config.ephemeral, owner, root, tree)?;

    Ok(())
}

fn add_paths_to_tree<'a>(
    paths: &Paths,
    owner: Owner<'a>,
    root: &Path,
    tree: &mut Tree<'a>,
) -> eyre::Result<()> {
    for directory in &paths.intermediate {
        tree.add_directory_path(owner, &root.join(directory))?;
    }
    for directory in &paths.directories {
        tree.add_entry_path(owner, &root.join(directory), EntryType::Directory)?;
    }
    for file in &paths.files {
        tree.add_entry_path(owner, &root.join(file), EntryType::File)?;
    }
    for symlink in &paths.symlinks {
        tree.add_entry_path(owner, &root.join(symlink), EntryType::Symlink)?;
    }

    Ok(())
}
