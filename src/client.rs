use crate::grpc::{
    status_client::StatusClient as GRPCStatusClient, zfs_client::ZfsClient as GRPCZfsClient,
    PingResult, ZfsListFilter, ZfsName,
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
