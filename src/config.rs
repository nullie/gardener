use std::{
    collections::BTreeMap,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::declarative::{self, FileType, PathType, StorageClass, tree::Tree};

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

impl Config {
    pub fn load() -> eyre::Result<Self> {
        let input = std::fs::File::open("/etc/gardener.json")?;
        let buffered = BufReader::new(input);

        Ok(serde_json::from_reader(buffered)?)
    }

    pub fn add_to_tree<'a>(&'a self, tree: &mut Tree<'a>) -> eyre::Result<()> {
        for (path, path_properties) in self.paths() {
            tree.add_path(
                path_properties.owner_module,
                &path,
                path_properties.path_type,
            )?;
        }

        Ok(())
    }

    pub fn paths(&self) -> impl Iterator<Item = (PathBuf, declarative::Properties<'_>)> {
        let user_paths = self.users.iter().flat_map(|(user_name, user_config)| {
            let home_dir = Path::new(&user_config.home);

            let adhoc_paths = user_config.adhoc.iter().flat_map(|(name, module)| {
                let owner_module = declarative::Owner::AdhocUser {
                    name,
                    user: user_name,
                };

                module_to_paths(module).map(move |(path, path_type, storage_class)| {
                    (
                        path,
                        declarative::Properties {
                            path_type,
                            owner_module,
                            storage_class,
                        },
                    )
                })
            });

            let paths = user_config.modules.iter().flat_map(|(name, &enabled)| {
                let module = self.available_modules.user.get(name).unwrap();
                let owner_module = declarative::Owner::User {
                    name,
                    user: user_name,
                    enabled,
                };

                module_to_paths(module).map(move |(path, path_type, storage_class)| {
                    (
                        path,
                        declarative::Properties {
                            path_type,
                            owner_module,
                            storage_class,
                        },
                    )
                })
            });

            adhoc_paths
                .chain(paths)
                .map(|(path, properties)| (home_dir.join(path), properties))
        });

        let system_paths = self.enabled_modules.iter().flat_map(|(name, &enabled)| {
            let module = self.available_modules.system.get(name).unwrap();
            let owner_module = declarative::Owner::System { name, enabled };

            module_to_paths(module).map(move |(path, path_type, storage_class)| {
                (
                    path.to_owned(),
                    declarative::Properties {
                        path_type,
                        owner_module,
                        storage_class,
                    },
                )
            })
        });

        user_paths.chain(system_paths)
    }
}

fn module_to_paths(module: &Module) -> impl Iterator<Item = (&Path, PathType, StorageClass)> {
    [
        (&module.cache, StorageClass::Cache),
        (&module.data, StorageClass::Data),
        (&module.ephemeral, StorageClass::Ephemeral),
    ]
    .into_iter()
    .flat_map(|(path_set, storage_class)| path_set_to_paths(path_set, storage_class))
}

fn path_set_to_paths(
    path_set: &Paths,
    storage_class: StorageClass,
) -> impl Iterator<Item = (&Path, PathType, StorageClass)> {
    [
        (
            &path_set.directories,
            PathType::ClosedDirectory,
            storage_class,
        ),
        (
            &path_set.files,
            PathType::File(FileType::Regular),
            storage_class,
        ),
        (
            &path_set.symlinks,
            PathType::File(FileType::Symlink),
            storage_class,
        ),
    ]
    .into_iter()
    .flat_map(|(paths, path_type, storage_class)| {
        paths
            .iter()
            .map(move |path| (path.as_ref(), path_type, storage_class))
    })
}
