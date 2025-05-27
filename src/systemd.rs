#![allow(dead_code)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use zbus_systemd::{systemd1::ManagerProxy, zbus::connection::Connection};

#[derive(Debug, Clone)]
pub(crate) struct Systemd {
    client: Connection,
}

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
}

impl Systemd {
    pub async fn new_session() -> Result<Self> {
        Ok(Self {
            client: Connection::session().await?,
        })
    }

    pub async fn new_system() -> Result<Self> {
        Ok(Self {
            client: Connection::system().await?,
        })
    }

    pub async fn list(&self) -> Result<Vec<Unit>> {
        let manager = ManagerProxy::new(&self.client).await?;
        let list = manager.list_units().await?;
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
            })
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::systemd::{LastRunState, Systemd};

    #[tokio::test]
    async fn test_list() {
        let systemd = Systemd::new_system().await.unwrap();
        let list = systemd.list().await.unwrap();
        for item in list {
            // on any sane system this should be running
            if item.name == "default.target" {
                assert_eq!(item.last_run_state, LastRunState::Running);
            }
        }
    }
}
