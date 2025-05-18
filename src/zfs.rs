#![allow(dead_code)]
use crate::grpc::{ZfsDataset, ZfsEntry, ZfsList, ZfsType, ZfsVolume};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    CreateDataset(Dataset),
    CreateVolume(Volume),
    Destroy(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ZFSKind {
    Dataset,
    Volume,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dataset {
    pub name: String,
    pub quota: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Volume {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Pool {
    name: String,
    controller: Controller,
}

#[derive(Debug, Clone)]
pub struct ZFSStat {
    pub kind: ZFSKind,
    pub name: String,
    pub full_name: String,
    pub size: u64,
    pub used: u64,
    pub avail: u64,
    pub refer: u64,
    pub mountpoint: Option<String>,
    // FIXME collect options (like quotas)
}

impl Into<ZfsDataset> for Dataset {
    fn into(self) -> ZfsDataset {
        ZfsDataset {
            name: self.name,
            quota: self.quota,
        }
    }
}

impl From<ZfsDataset> for Dataset {
    fn from(value: ZfsDataset) -> Self {
        Self {
            name: value.name,
            quota: value.quota,
        }
    }
}

impl Into<ZfsVolume> for Volume {
    fn into(self) -> ZfsVolume {
        ZfsVolume {
            name: self.name,
            size: self.size,
        }
    }
}

impl From<ZfsVolume> for Volume {
    fn from(value: ZfsVolume) -> Self {
        Self {
            name: value.name,
            size: value.size,
        }
    }
}

impl From<ZfsList> for Vec<ZFSStat> {
    fn from(value: ZfsList) -> Self {
        let mut list = Self::default();
        for item in value.entries {
            list.push(item.into())
        }
        list
    }
}

impl Into<ZfsList> for Vec<ZFSStat> {
    fn into(self) -> ZfsList {
        let mut list = ZfsList::default();
        for item in self {
            list.entries.push(item.into())
        }
        list
    }
}

impl From<ZfsEntry> for ZFSStat {
    fn from(value: ZfsEntry) -> Self {
        Self {
            kind: match value.kind() {
                ZfsType::Volume => ZFSKind::Volume,
                ZfsType::Dataset => ZFSKind::Dataset,
            }
            .into(),
            name: value.name,
            full_name: value.full_name,
            size: value.size,
            used: value.used,
            avail: value.avail,
            refer: value.refer,
            mountpoint: value.mountpoint,
        }
    }
}

impl Into<ZfsEntry> for ZFSStat {
    fn into(self) -> ZfsEntry {
        ZfsEntry {
            kind: match self.kind {
                ZFSKind::Volume => ZfsType::Volume,
                ZFSKind::Dataset => ZfsType::Dataset,
            }
            .into(),
            name: self.name,
            full_name: self.full_name,
            size: self.size,
            used: self.used,
            avail: self.avail,
            refer: self.refer,
            mountpoint: self.mountpoint,
        }
    }
}

impl Pool {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            controller: Controller::default(),
        }
    }

    pub fn create_dataset(&self, info: &Dataset) -> Result<()> {
        let mut options: Option<CommandOptions> = None;

        if let Some(quota) = &info.quota {
            let mut tmp = CommandOptions::default();
            tmp.insert("quota".to_string(), quota.clone());
            options = Some(tmp);
        }

        self.controller
            .create_dataset(&self.name, &info.name, options)?;
        Ok(())
    }

    pub fn create_volume(&self, info: &Volume) -> Result<()> {
        self.controller
            .create_volume(&self.name, &info.name, info.size, None)?;
        Ok(())
    }

    pub fn destroy(&self, name: String) -> Result<()> {
        self.controller.destroy(&self.name, &name)?;
        Ok(())
    }

    pub fn list(&self, filter: Option<String>) -> Result<Vec<ZFSStat>> {
        let mut ret = Vec::new();
        let list_output = self.controller.list()?;
        let lines = list_output.split('\n');
        for line in lines.skip(1) {
            if line.is_empty() {
                continue;
            }

            let mut name = String::new();
            let mut used = String::new();
            let mut avail = String::new();
            let mut refer = String::new();

            let mut tmp = String::new();
            let mut stage = 0;
            'line: for ch in line.chars() {
                if ch != ' ' {
                    tmp.push(ch)
                } else if tmp != "" {
                    match stage {
                        0 => name = tmp,
                        1 => used = tmp,
                        2 => avail = tmp,
                        3 => refer = tmp,
                        _ => break 'line,
                    };
                    stage += 1;
                    tmp = String::new();
                }
            }

            let mountpoint = tmp;

            if let Some(filter) = &filter {
                if !name.starts_with(&format!("{}/{}", self.name, filter)) {
                    continue;
                }
            }

            if !name.starts_with(&self.name) {
                continue;
            }

            if name == self.name {
                // skip root-level datasets since they correspond to pools
                continue;
            }

            ret.push(ZFSStat {
                // volumes don't have a mountpath, '-' is indicated
                // FIXME relying on datasets being mounted is a thing we're doing right now, it'll
                //       probably have to change eventually, but zfs handles all the mounting for
                //       us at create and destroy time.
                kind: if mountpoint == "-" {
                    ZFSKind::Volume
                } else {
                    ZFSKind::Dataset
                },
                full_name: name.clone(),
                name: name
                    .strip_prefix(&format!("{}/", self.name))
                    .unwrap_or_else(|| &name)
                    .to_owned(), // strip the pool
                used: used.parse()?,
                avail: avail.parse()?,
                // this is just easier to use in places
                size: if mountpoint == "-" {
                    used.parse()?
                } else {
                    used.parse::<u64>()? + avail.parse::<u64>()?
                },
                refer: refer.parse()?,
                mountpoint: if mountpoint == "-" {
                    None
                } else {
                    Some(mountpoint.to_string())
                },
            })
        }
        Ok(ret)
    }
}

#[derive(Debug, Clone, Default)]
struct CommandOptions(HashMap<String, String>);

impl CommandOptions {
    fn to_options(&self) -> Vec<String> {
        let mut args = Vec::new();
        for (key, value) in &self.0 {
            args.push("-o".to_string());
            args.push(format!("{}={}", key, value));
        }
        args
    }
}

impl std::ops::Deref for CommandOptions {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CommandOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default)]
struct Controller;

impl Controller {
    fn run(command: &str, args: Vec<String>) -> Result<String> {
        let out = std::process::Command::new(command)
            .args(args.clone())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;
        if out.status.success() {
            Ok(String::from_utf8(out.stdout)?)
        } else {
            Err(anyhow!(
                "Error: {}",
                String::from_utf8(out.stderr)?.as_str()
            ))
        }
    }

    fn list(&self) -> Result<String> {
        Self::run("zfs", vec!["list".to_string(), "-p".to_string()])
    }

    fn destroy(&self, pool: &str, name: &str) -> Result<()> {
        Self::run(
            "zfs",
            vec!["destroy".to_string(), format!("{}/{}", pool, name)],
        )?;
        Ok(())
    }

    fn create_dataset(
        &self,
        pool: &str,
        name: &str,
        options: Option<CommandOptions>,
    ) -> Result<()> {
        let mut args = vec!["create".to_string(), format!("{}/{}", pool, name)];

        if let Some(options) = options {
            args.append(&mut options.to_options())
        }

        Self::run("zfs", args)?;
        Ok(())
    }

    fn create_volume(
        &self,
        pool: &str,
        name: &str,
        size: u64, // 640k aughta be enough for anybody
        options: Option<CommandOptions>,
    ) -> Result<()> {
        let mut args = vec![
            "create".to_string(),
            "-V".to_string(),
            format!("{}", size),
            format!("{}/{}", pool, name),
        ];

        if let Some(options) = options {
            args.append(&mut options.to_options())
        }

        Self::run("zfs", args)?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "zfs")]
mod tests {
    mod controller {
        use super::super::Pool;
        use crate::{
            testutil::{create_zpool, destroy_zpool, BUCKLE_TEST_ZPOOL_PREFIX},
            zfs::ZFSKind,
        };
        #[test]
        fn test_controller_zfs_lifecycle() {
            let _ = destroy_zpool("controller-list", None);
            let file = create_zpool("controller-list").unwrap();
            let pool = Pool::new(&format!("{}-controller-list", BUCKLE_TEST_ZPOOL_PREFIX));
            let list = pool.list(None).unwrap();
            assert_eq!(list.len(), 0);
            pool.create_dataset(&crate::zfs::Dataset {
                name: "dataset".to_string(),
                quota: None,
            })
            .unwrap();
            let list = pool.list(None).unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].kind, ZFSKind::Dataset);
            assert_eq!(list[0].name, "dataset");
            assert_eq!(
                list[0].full_name,
                format!("{}-controller-list/dataset", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(list[0].size, 0);
            assert_ne!(list[0].used, 0);
            assert_ne!(list[0].refer, 0);
            assert_ne!(list[0].avail, 0);
            assert_eq!(
                list[0].mountpoint,
                Some(format!(
                    "/{}-controller-list/dataset",
                    BUCKLE_TEST_ZPOOL_PREFIX
                ))
            );
            pool.create_volume(&crate::zfs::Volume {
                name: "volume".to_string(),
                size: 100 * 1024 * 1024,
            })
            .unwrap();
            let list = pool.list(None).unwrap();
            assert_eq!(list.len(), 2);
            let list = pool.list(Some("volume".to_string())).unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].kind, ZFSKind::Volume);
            assert_eq!(list[0].name, "volume");
            assert_eq!(
                list[0].full_name,
                format!("{}-controller-list/volume", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(list[0].size, 0);
            assert_ne!(list[0].used, 0);
            assert_ne!(list[0].refer, 0);
            assert_ne!(list[0].avail, 0);
            assert_eq!(list[0].mountpoint, None);
            let list = pool.list(Some("dataset".to_string())).unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].kind, ZFSKind::Dataset);
            assert_eq!(list[0].name, "dataset");
            assert_eq!(
                list[0].full_name,
                format!("{}-controller-list/dataset", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(list[0].size, 0);
            assert_ne!(list[0].used, 0);
            assert_ne!(list[0].refer, 0);
            assert_ne!(list[0].avail, 0);
            assert_eq!(
                list[0].mountpoint,
                Some(format!(
                    "/{}-controller-list/dataset",
                    BUCKLE_TEST_ZPOOL_PREFIX
                ))
            );
            pool.destroy("dataset".to_string()).unwrap();
            let list = pool.list(Some("dataset".to_string())).unwrap();
            assert_eq!(list.len(), 0);
            let list = pool.list(None).unwrap();
            assert_eq!(list.len(), 1);
            pool.destroy("volume".to_string()).unwrap();
            let list = pool.list(Some("volume".to_string())).unwrap();
            assert_eq!(list.len(), 0);
            let list = pool.list(None).unwrap();
            assert_eq!(list.len(), 0);
            destroy_zpool("controller-list", Some(&file)).unwrap();
        }
    }
}
