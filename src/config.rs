use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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
