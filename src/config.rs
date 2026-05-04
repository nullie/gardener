use std::{
    collections::BTreeMap,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::declarative::{DeclaredFileType, DeclaredPathType, tree::Tree};

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
}

impl Config {
    pub fn load() -> eyre::Result<Self> {
        let input = std::fs::File::open("/etc/gardener.json")?;
        let buffered = BufReader::new(input);

        Ok(serde_json::from_reader(buffered)?)
    }

    pub fn add_to_tree<'a>(&'a self, tree: &mut Tree<'a>) -> eyre::Result<()> {
        for (owner, path, path_type) in self.paths() {
            tree.add_path(owner, &path, path_type)?;
        }

        Ok(())
    }

    fn paths(&self) -> impl Iterator<Item = (OwnerModule<'_>, PathBuf, DeclaredPathType)> {
        let user_paths = self.users.iter().flat_map(|(user_name, user_config)| {
            let home_dir = Path::new(&user_config.home);

            let adhoc_paths = user_config.adhoc.iter().flat_map(|(name, module)| {
                let owner_module = OwnerModule::AdhocUser {
                    name,
                    user: user_name,
                };

                module_to_paths(module)
                    .map(move |(path, file_type)| (owner_module, path, file_type))
            });

            let paths = user_config.modules.iter().flat_map(|(name, &enabled)| {
                let module = self.available_modules.user.get(name).unwrap();
                let owner_module = OwnerModule::User {
                    name,
                    user: user_name,
                    enabled,
                };

                module_to_paths(module)
                    .map(move |(path, file_type)| (owner_module, path, file_type))
            });

            adhoc_paths
                .chain(paths)
                .map(|(owner_module, path, file_type)| {
                    (owner_module, home_dir.join(path), file_type)
                })
        });

        let system_paths = self.enabled_modules.iter().flat_map(|(name, &enabled)| {
            let module = self.available_modules.system.get(name).unwrap();
            let owner_module = OwnerModule::System { name, enabled };

            module_to_paths(module)
                .map(move |(path, file_type)| (owner_module, path.to_owned(), file_type))
        });

        user_paths.chain(system_paths)
    }
}

fn module_to_paths(module: &Module) -> impl Iterator<Item = (&Path, DeclaredPathType)> {
    [&module.cache, &module.data, &module.ephemeral]
        .into_iter()
        .flat_map(path_set_to_paths)
}

fn path_set_to_paths(path_set: &Paths) -> impl Iterator<Item = (&Path, DeclaredPathType)> {
    [
        (&path_set.directories, DeclaredPathType::ClosedDirectory),
        (
            &path_set.files,
            DeclaredPathType::File(DeclaredFileType::Regular),
        ),
        (
            &path_set.symlinks,
            DeclaredPathType::File(DeclaredFileType::Symlink),
        ),
    ]
    .into_iter()
    .flat_map(|(paths, path_type)| paths.iter().map(move |path| (path.as_ref(), path_type)))
}
