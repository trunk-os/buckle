#![allow(dead_code)]
use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;

pub enum Operation {
    CreateDataset(Dataset),
    CreateVolume(Volume),
}

pub struct Dataset {
    pub name: String,
    pub quota: Option<String>,
}

pub struct Volume {
    pub name: String,
    pub size: u64,
}

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

    pub fn run(&self, operation: Operation) -> Result<()> {
        match operation {
            Operation::CreateDataset(ds) => {
                let mut options: Option<CommandOptions> = None;

                if let Some(quota) = ds.quota {
                    let mut tmp = CommandOptions::default();
                    tmp.insert("quota".to_string(), quota);
                    options = Some(tmp);
                }

                self.controller
                    .create_dataset(&self.name, &ds.name, options)
            }
            Operation::CreateVolume(vl) => self
                .controller
                .create_volume(&self.name, &vl.name, vl.size, None),
        }
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
