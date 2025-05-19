use crate::grpc::{
    status_client::StatusClient as GRPCStatusClient, zfs_client::ZfsClient as GRPCZfsClient,
    ZfsListFilter, ZfsName,
};

pub use crate::zfs::{Dataset, Volume, ZFSStat}; // we expose these types we should serve them
use anyhow::Result;
use std::path::PathBuf;
use tonic::{transport::Channel, Request};

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
    pub fn new(socket: PathBuf) -> Result<Self> {
        Ok(Self { socket })
    }

    pub async fn status(&self) -> Result<StatusClient> {
        let client =
            GRPCStatusClient::connect(format!("unix://{}", self.socket.to_str().unwrap())).await?;
        Ok(StatusClient { client })
    }

    pub async fn zfs(&self) -> Result<ZFSClient> {
        let client =
            GRPCZfsClient::connect(format!("unix://{}", self.socket.to_str().unwrap())).await?;
        Ok(ZFSClient { client })
    }
}

impl StatusClient {
    pub async fn ping(&mut self) -> Result<()> {
        self.client.ping(Request::new(())).await?;
        Ok(())
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
