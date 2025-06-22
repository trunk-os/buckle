use crate::{
    grpc::{
        status_client::StatusClient as GRPCStatusClient,
        systemd_client::SystemdClient as GRPCSystemdClient, zfs_client::ZfsClient as GRPCZfsClient,
        PingResult, UnitEnabledState, UnitRuntimeState, UnitSettings as GRPCUnitSettings,
        ZfsListFilter, ZfsName,
    },
    systemd::{EnabledState, RuntimeState, UnitSettings},
};
// we expose these types we should serve them
pub use crate::{
    sysinfo::Info,
    zfs::{Dataset, ModifyDataset, ModifyVolume, Volume, ZFSStat},
};
use std::path::PathBuf;
use tonic::{transport::Channel, Request};

type Result<T> = std::result::Result<T, tonic::Status>;

#[derive(Debug, Clone)]
pub struct Client {
    socket: PathBuf,
}

pub struct StatusClient {
    client: GRPCStatusClient<Channel>,
}

pub struct ZFSClient {
    client: GRPCZfsClient<Channel>,
}

pub struct SystemdClient {
    client: GRPCSystemdClient<Channel>,
}

impl Client {
    pub fn new(socket: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { socket })
    }

    pub async fn status(&self) -> anyhow::Result<StatusClient> {
        let client =
            GRPCStatusClient::connect(format!("unix://{}", self.socket.to_str().unwrap())).await?;
        Ok(StatusClient { client })
    }

    pub async fn zfs(&self) -> anyhow::Result<ZFSClient> {
        let client =
            GRPCZfsClient::connect(format!("unix://{}", self.socket.to_str().unwrap())).await?;
        Ok(ZFSClient { client })
    }

    pub async fn systemd(&self) -> anyhow::Result<SystemdClient> {
        let client =
            GRPCSystemdClient::connect(format!("unix://{}", self.socket.to_str().unwrap())).await?;
        Ok(SystemdClient { client })
    }
}

impl SystemdClient {
    pub async fn list(&mut self) -> Result<Vec<UnitSettings>> {
        let units = self.client.list(Request::new(())).await?.into_inner();
        let mut v = Vec::new();
        for unit in &units.items {
            let u = UnitSettings {
                name: unit.name.to_string(),
                enabled_state: match unit.enabled_state() {
                    UnitEnabledState::Enabled => EnabledState::Enabled,
                    UnitEnabledState::Disabled => EnabledState::Disabled,
                    UnitEnabledState::Failed => EnabledState::Failed,
                },
                runtime_state: match unit.runtime_state() {
                    UnitRuntimeState::Started => RuntimeState::Started,
                    UnitRuntimeState::Stopped => RuntimeState::Stopped,
                    UnitRuntimeState::Reloaded => RuntimeState::Reloaded,
                    UnitRuntimeState::Restarted => RuntimeState::Restarted,
                },
            };
            v.push(u)
        }

        Ok(v)
    }

    pub async fn set_unit(&mut self, unit: UnitSettings) -> Result<()> {
        let out = GRPCUnitSettings {
            name: unit.name,
            enabled_state: match unit.enabled_state {
                EnabledState::Enabled => UnitEnabledState::Enabled,
                EnabledState::Disabled => UnitEnabledState::Disabled,
                EnabledState::Failed => UnitEnabledState::Failed,
                _ => {
                    return Err(tonic::Status::new(
                        tonic::Code::Internal,
                        format!("Invalid state '{}'", unit.enabled_state),
                    ))
                }
            }
            .into(),
            runtime_state: match unit.runtime_state {
                RuntimeState::Started => UnitRuntimeState::Started,
                RuntimeState::Stopped => UnitRuntimeState::Stopped,
                RuntimeState::Reloaded => UnitRuntimeState::Reloaded,
                RuntimeState::Restarted => UnitRuntimeState::Restarted,
                _ => {
                    return Err(tonic::Status::new(
                        tonic::Code::Internal,
                        format!("Invalid state '{}'", unit.enabled_state),
                    ))
                }
            }
            .into(),
        };
        self.client.set_unit(Request::new(out)).await?;
        Ok(())
    }
}

impl StatusClient {
    pub async fn ping(&mut self) -> Result<PingResult> {
        Ok(self.client.ping(Request::new(())).await?.into_inner())
    }
}

impl ZFSClient {
    pub async fn create_dataset(&mut self, dataset: Dataset) -> Result<()> {
        self.client
            .create_dataset(Request::new(dataset.into()))
            .await?;
        Ok(())
    }

    pub async fn create_volume(&mut self, volume: Volume) -> Result<()> {
        self.client
            .create_volume(Request::new(volume.into()))
            .await?;
        Ok(())
    }

    pub async fn modify_dataset(&mut self, dataset: ModifyDataset) -> Result<()> {
        self.client
            .modify_dataset(Request::new(dataset.into()))
            .await?;
        Ok(())
    }

    pub async fn modify_volume(&mut self, volume: ModifyVolume) -> Result<()> {
        self.client
            .modify_volume(Request::new(volume.into()))
            .await?;
        Ok(())
    }

    pub async fn list(&mut self, filter: Option<String>) -> Result<Vec<ZFSStat>> {
        Ok(self
            .client
            .list(Request::new(ZfsListFilter { filter }))
            .await?
            .into_inner()
            .into())
    }

    pub async fn destroy(&mut self, name: String) -> Result<()> {
        self.client.destroy(Request::new(ZfsName { name })).await?;
        Ok(())
    }
}
