#![allow(dead_code)]
use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default)]
struct CommandOptions(HashMap<String, String>);

impl Into<Vec<String>> for CommandOptions {
    fn into(self) -> Vec<String> {
        let mut args = Vec::new();
        for (key, value) in self.0 {
            args.push("-o".to_string());
            args.push(format!("{}={}", key, value));
        }
        args
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

pub struct Pool {
    name: String,
}

impl Pool {
    pub fn new(name: String) -> Self {
        Self { name }
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
        pool: String,
        name: String,
        options: Option<CommandOptions>,
    ) -> Result<()> {
        let mut args = vec!["create".to_string(), format!("{}/{}", pool, name)];

        if let Some(options) = options {
            args.append(&mut options.into())
        }

        Self::run(&self.zfs_path, args)?;
        Ok(())
    }

    fn create_volume(
        &self,
        pool: String,
        name: String,
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
            args.append(&mut options.into())
        }

        Self::run(&self.zfs_path, args)?;
        Ok(())
    }
}
