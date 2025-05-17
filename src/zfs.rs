#![allow(dead_code)]
use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;

pub enum Operation {
    CreateDataset(Dataset),
    CreateVolume(Volume),
    Destroy(String),
}

#[derive(Debug, Clone)]
pub struct Dataset {
    pub name: String,
    pub quota: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Pool {
    name: String,
    controller: Controller,
}

impl Pool {
    pub fn new(name: String) -> Self {
        Self {
            name,
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
}

static ZPOOLPATH: LazyLock<String> = LazyLock::new(|| {
    String::from_utf8(
        std::process::Command::new("which")
            .args(vec!["zpool"])
            .output()
            .expect("finding location of zfs command")
            .stdout,
    )
    .expect("check UTF-8 validity")
});

static ZFSPATH: LazyLock<String> = LazyLock::new(|| {
    String::from_utf8(
        std::process::Command::new("which")
            .args(vec!["zfs"])
            .output()
            .expect("finding location of zfs command")
            .stdout,
    )
    .expect("check UTF-8 validity")
});

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

#[derive(Debug, Clone)]
struct Controller {
    zfs_path: String,
    zpool_path: String,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            zfs_path: ZFSPATH.clone(),
            zpool_path: ZPOOLPATH.clone(),
        }
    }
}

impl Controller {
    fn run(command: &str, args: Vec<String>) -> Result<String> {
        Ok(String::from_utf8(
            std::process::Command::new(command)
                .args(args)
                .output()?
                .stdout,
        )?)
    }

    fn destroy(&self, pool: &str, name: &str) -> Result<()> {
        Self::run(
            &self.zfs_path,
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

        Self::run(&self.zfs_path, args)?;
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

        Self::run(&self.zfs_path, args)?;
        Ok(())
    }
}
