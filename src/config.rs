use crate::zfs::Pool;
use anyhow::Result;
use serde::Deserialize;

pub(crate) const CONFIG_PATH: &str = "/trunk/config.yaml";
pub(crate) const DEFAULT_ZPOOL: &str = "trunk";

fn default_zpool() -> String {
    DEFAULT_ZPOOL.to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
    pub(crate) socket: std::path::PathBuf,
    pub(crate) zfs: ZFSConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ZFSConfig {
    #[serde(default = "default_zpool")]
    pub(crate) pool: String,
}

impl ZFSConfig {
    pub(crate) fn controller(&self) -> Pool {
        Pool::new(&self.pool)
    }
}

impl Config {
    pub fn from_file(filename: std::path::PathBuf) -> Result<Self> {
        let r = std::fs::OpenOptions::new().read(true).open(filename)?;
        Ok(serde_yaml_ng::from_reader(r)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_file(CONFIG_PATH.into()).expect("while reading config file")
    }
}
