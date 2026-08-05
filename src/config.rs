use std::{
    collections::BTreeMap,
    io::BufReader,
    path::{Path, PathBuf},
};

use rootcause::prelude::ResultExt;
use serde::{Deserialize, Deserializer};

use crate::declarative::{self, FileType, PathType, StorageClass, tree::Tree};

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
    pub fn load() -> rootcause::Result<Self> {
        let input = std::fs::File::open("/etc/gardener.json")?;
        let buffered = BufReader::new(input);

        Ok(serde_json::from_reader(buffered)?)
    }

    /// Convert to tree, adding sytstemd-tmpfiles
    pub fn to_tree(&self) -> rootcause::Result<declarative::Tree<'_>> {
        let mut tree = Tree::new();

        self.add_to_tree(&mut tree)
            .context("failed adding config paths")?;
        declarative::tmpfiles::add_systemd_tmpfiles(&mut tree).context("failed adding tmpfiles")?;

        Ok(tree)
    }

    pub fn add_to_tree<'a>(&'a self, tree: &mut declarative::Tree<'a>) -> rootcause::Result<()> {
        for (path, path_type, path_properties) in self.paths() {
            tree.add_path(&path, path_type, path_properties)
                .map_err(|e| rootcause::report!(e.to_string()))?;
        }

        Ok(())
    }

    pub fn paths(&self) -> impl Iterator<Item = (PathBuf, PathType, declarative::Properties<'_>)> {
        let user_paths = self.users.iter().flat_map(|(user_name, user_config)| {
            let home_dir = Path::new(&user_config.home);

            let adhoc_paths = user_config.adhoc.iter().flat_map(|(name, module)| {
                let owner = declarative::Owner::AdhocUser {
                    name,
                    user: user_name,
                };

                module_to_paths(module, owner)
            });

            let paths = user_config.modules.iter().flat_map(|(name, &enabled)| {
                let module = self.available_modules.user.get(name).unwrap();
                let owner = declarative::Owner::User {
                    name,
                    user: user_name,
                    enabled,
                };

                module_to_paths(module, owner)
            });

            adhoc_paths
                .chain(paths)
                .map(|(path, path_type, properties)| (home_dir.join(path), path_type, properties))
        });

        let system_paths = self.enabled_modules.iter().flat_map(|(name, &enabled)| {
            let module = self.available_modules.system.get(name).unwrap();
            let owner = declarative::Owner::System { name, enabled };

            module_to_paths(module, owner)
                .map(move |(path, path_type, properties)| (path.to_owned(), path_type, properties))
        });

        user_paths.chain(system_paths)
    }
}

fn module_to_paths<'a>(
    module: &'a Module,
    owner: declarative::Owner<'a>,
) -> impl Iterator<Item = (&'a Path, PathType, declarative::Properties<'a>)> {
    [
        (&module.cache, StorageClass::Cache),
        (&module.data, StorageClass::Data),
        (&module.ephemeral, StorageClass::Ephemeral),
    ]
    .into_iter()
    .flat_map(move |(path_set, storage_class)| {
        path_set_to_paths(
            path_set,
            declarative::Properties {
                owner,
                storage_class,
            },
        )
    })
}

fn path_set_to_paths<'a>(
    path_set: &'a Paths,
    properties: declarative::Properties<'a>,
) -> impl Iterator<Item = (&'a Path, PathType, declarative::Properties<'a>)> {
    [
        (
            &path_set.directories,
            PathType::Directory {
                owns_contents: true,
            },
        ),
        (&path_set.files, PathType::File(FileType::Regular)),
        (&path_set.symlinks, PathType::File(FileType::Symlink)),
    ]
    .into_iter()
    .flat_map(move |(paths, path_type)| {
        paths
            .iter()
            .map(move |path| (path.as_ref(), path_type, properties))
    })
}
