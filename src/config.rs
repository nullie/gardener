use std::{
    collections::BTreeMap,
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
    pub available_modules: AvailableModules,
    pub enabled_modules: EnabledModules,
    pub users: BTreeMap<String, UserConfig>,
}

#[derive(Deserialize, Debug)]
pub struct AvailableModules {
    pub user: BTreeMap<String, Module>,
    pub system: BTreeMap<String, Module>,
}

type EnabledModules = BTreeMap<String, bool>;

#[derive(Deserialize, Debug)]
pub struct UserConfig {
    pub adhoc: BTreeMap<String, Module>,
    pub home: String,
    pub modules: EnabledModules,
}

#[derive(Deserialize, Debug)]
pub struct Module {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub cache: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub data: Paths,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub ephemeral: Paths,
}

#[derive(Deserialize, Debug, Default)]
pub struct Paths {
    pub directories: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub symlinks: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
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
        for (user_name, user_config) in &self.users {
            let home_dir = Path::new(&user_config.home);

            for (name, module) in &user_config.adhoc {
                add_module_to_tree(
                    module,
                    OwnerModule::AdhocUser {
                        name,
                        user: user_name,
                    },
                    home_dir,
                    tree,
                )?;
            }

            for (name, &enabled) in &user_config.modules {
                let module = self.available_modules.user.get(name).unwrap();
                add_module_to_tree(
                    module,
                    OwnerModule::User {
                        name,
                        user: user_name,
                        enabled,
                    },
                    home_dir,
                    tree,
                )?;
            }
        }

        let root = Path::new("/");

        for (name, &enabled) in &self.enabled_modules {
            let module = self.available_modules.system.get(name).unwrap();
            add_module_to_tree(module, OwnerModule::System { name, enabled }, root, tree)?;
        }

        Ok(())
    }
}

fn add_module_to_tree<'a>(
    module: &Module,
    owner: OwnerModule<'a>,
    root: &Path,
    tree: &mut Tree<'a>,
) -> eyre::Result<()> {
    add_paths_to_tree(&module.cache, owner, root, tree)?;
    add_paths_to_tree(&module.data, owner, root, tree)?;
    add_paths_to_tree(&module.ephemeral, owner, root, tree)?;

    Ok(())
}

fn add_paths_to_tree<'a>(
    paths: &Paths,
    owner: OwnerModule<'a>,
    root: &Path,
    tree: &mut Tree<'a>,
) -> eyre::Result<()> {
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
