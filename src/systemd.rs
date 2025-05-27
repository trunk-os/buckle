#![allow(dead_code)]
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use zbus_systemd::zbus::connection::Connection;

#[derive(Debug, Clone)]
pub(crate) struct Systemd {
    client: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeState {
    Started,
    Stopped,
    Restarted,
    Reloaded,
}

impl ToString for RuntimeState {
    fn to_string(&self) -> String {
        match self {
            Self::Started => "started",
            Self::Stopped => "stopped",
            Self::Restarted => "restarted",
            Self::Reloaded => "reloaded",
        }
        .into()
    }
}

impl std::str::FromStr for RuntimeState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "started" => Self::Started,
            "stopped" => Self::Stopped,
            "restarted" => Self::Restarted,
            "reloaded" => Self::Reloaded,
            _ => return Err(anyhow!("invalid runtime state")),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnabledState {
    Enabled,
    Disabled,
}

impl ToString for EnabledState {
    fn to_string(&self) -> String {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
        .into()
    }
}

impl std::str::FromStr for EnabledState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "enabled" => Self::Enabled,
            "disabled" => Self::Disabled,
            _ => return Err(anyhow!("invalid enabled state")),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSettings {
    pub name: String,
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

    pub async fn list(&self) -> Result<()> {
        Ok(())
    }
}
