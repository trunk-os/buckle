#![allow(dead_code)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use zbus_systemd::{
    systemd1::{ManagerProxy, UnitProxy},
    zbus::connection::Connection,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum LastRunState {
    Failed,
    Dead,
    Mounted,
    Running,
    Listening,
    Plugged,
    Exited,
    Active,
    Waiting,
    Unknown(String),
}

impl ToString for LastRunState {
    fn to_string(&self) -> String {
        match self {
            Self::Failed => "failed",
            Self::Active => "active",
            Self::Dead => "dead",
            Self::Mounted => "mounted",
            Self::Running => "running",
            Self::Listening => "listening",
            Self::Plugged => "plugged",
            Self::Exited => "exited",
            Self::Waiting => "waiting",
            Self::Unknown(s) => &s,
        }
        .into()
    }
}

impl std::str::FromStr for LastRunState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "failed" => Self::Failed,
            "dead" => Self::Dead,
            "mounted" => Self::Mounted,
            "running" => Self::Running,
            "listening" => Self::Listening,
            "plugged" => Self::Plugged,
            "exited" => Self::Exited,
            "active" => Self::Active,
            "waiting" => Self::Waiting,
            s => Self::Unknown(s.to_string()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum RuntimeState {
    Started,
    Stopped,
    Restarted,
    Reloaded,
    Unknown(String),
}

impl ToString for RuntimeState {
    fn to_string(&self) -> String {
        match self {
            Self::Started => "started",
            Self::Stopped => "stopped",
            Self::Restarted => "restarted",
            Self::Reloaded => "reloaded",
            Self::Unknown(s) => &s,
        }
        .into()
    }
}

impl std::str::FromStr for RuntimeState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "started" | "running" | "mounted" | "listening" | "plugged" | "active" => Self::Started,
            "stopped" | "dead" | "failed" | "exited" | "waiting" => Self::Stopped,
            "restarted" => Self::Restarted,
            "reloaded" => Self::Reloaded,
            s => Self::Unknown(s.to_string()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum EnabledState {
    Enabled,
    Disabled,
    Failed,
    Unknown(String),
}

impl ToString for EnabledState {
    fn to_string(&self) -> String {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Failed => "failed",
            Self::Unknown(s) => &s,
        }
        .into()
    }
}

impl std::str::FromStr for EnabledState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "enabled" | "active" => Self::Enabled,
            "disabled" | "inactive" => Self::Disabled,
            "failed" => Self::Failed,
            s => Self::Unknown(s.to_string()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct UnitSettings {
    pub name: String,
    pub enabled_state: EnabledState,
    pub runtime_state: RuntimeState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Unit {
    pub name: String,
    pub description: String,
    pub last_run_state: LastRunState,
    pub enabled_state: EnabledState,
    pub runtime_state: RuntimeState,
    pub object_path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Systemd {
    client: Connection,
    manager: ManagerProxy<'static>,
}

impl Systemd {
    pub async fn new(client: Connection) -> Result<Self> {
        Ok(Self {
            manager: ManagerProxy::new(&client).await?,
            client,
        })
    }

    pub async fn new_session() -> Result<Self> {
        Self::new(Connection::session().await?).await
    }

    pub async fn new_system() -> Result<Self> {
        Self::new(Connection::system().await?).await
    }

    pub async fn start(&self, name: String) -> Result<()> {
        self.manager.start_unit(name, "fail".into()).await?;
        Ok(())
    }

    pub async fn stop(&self, name: String) -> Result<()> {
        self.manager.stop_unit(name, "fail".into()).await?;
        Ok(())
    }

    pub async fn restart(&self, name: String) -> Result<()> {
        self.manager.restart_unit(name, "fail".into()).await?;
        Ok(())
    }

    pub async fn reload(&self, name: String) -> Result<()> {
        self.manager.reload_unit(name, "fail".into()).await?;
        Ok(())
    }

    pub async fn status(&self, name: String) -> Result<RuntimeState> {
        let service = UnitProxy::new(&self.client, name).await?;
        Ok(service.active_state().await?.parse()?)
    }

    pub async fn list(&self) -> Result<Vec<Unit>> {
        let list = self.manager.list_units().await?;
        let mut v = Vec::new();
        for item in list {
            let name = item.0;
            let description = item.1;
            let enabled_state: EnabledState = item.3.parse()?;

            // two kinds of data from one string
            let runtime_state: RuntimeState = item.4.parse()?;
            let last_run_state: LastRunState = item.4.parse()?;

            v.push(Unit {
                name,
                description,
                enabled_state,
                runtime_state,
                last_run_state,
                object_path: item.6.to_string(),
            })
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::systemd::{LastRunState, RuntimeState, Systemd};

    #[tokio::test]
    async fn test_status() {
        let systemd = Systemd::new_system().await.unwrap();
        let list = systemd.list().await.unwrap();
        let mut op = None;
        for item in list {
            // this should be running on any system that tests with zfs
            if item.name == "zfs-import.target" {
                op = Some(item.object_path)
            }
        }
        assert!(op.is_some(), "did not find item in systemd to check");
        let op = op.unwrap();

        let status = systemd.status(op).await.unwrap();
        assert_eq!(status, RuntimeState::Started);
    }

    #[tokio::test]
    async fn test_list() {
        let systemd = Systemd::new_system().await.unwrap();
        let list = systemd.list().await.unwrap();
        let mut found = false;
        for item in list {
            // on any sane system this should be running
            if item.name == "zfs-import.target" {
                assert_eq!(item.last_run_state, LastRunState::Active);
                found = true;
            }
        }
        assert!(found, "did not find item in systemd to check")
    }
}
