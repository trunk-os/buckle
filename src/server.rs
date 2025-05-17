use std::net::SocketAddr;

use crate::grpc::{
    status_server::{Status, StatusServer},
    zfs_server::{Zfs, ZfsServer},
    ZfsDataset, ZfsList, ZfsListFilter, ZfsName, ZfsVolume,
};
use tonic::{transport::Server as TransportServer, Request, Response, Result};

// FIXME needs a way to shut down
#[derive(Debug, Default, Clone)]
pub struct Server {
    config: crate::config::Config,
}

impl Server {
    #[cfg(test)]
    pub(crate) fn new_with_config(config: Option<crate::config::Config>) -> Self {
        match config {
            Some(config) => Self { config },
            None => Self::default(),
        }
    }

    pub fn start(
        &self,
        addr: SocketAddr,
    ) -> impl std::future::Future<Output = Result<(), tonic::transport::Error>> {
        TransportServer::builder()
            .add_service(StatusServer::new(self.clone()))
            .add_service(ZfsServer::new(self.clone()))
            .serve(addr)
    }
}

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<()>, tonic::Status> {
        return Ok(Response::new(()));
    }
}

#[tonic::async_trait]
impl Zfs for Server {
    async fn list(&self, filter: Request<ZfsListFilter>) -> Result<Response<ZfsList>> {
        let list = self
            .config
            .zfs
            .controller()
            .list(filter.get_ref().filter.clone())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        return Ok(Response::new(list.into()));
    }

    async fn create_dataset(
        &self,
        dataset: Request<ZfsDataset>,
    ) -> Result<Response<()>, tonic::Status> {
        self.config
            .zfs
            .controller()
            .create_dataset(&dataset.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;

        return Ok(Response::new(()));
    }

    async fn create_volume(
        &self,
        volume: Request<ZfsVolume>,
    ) -> Result<Response<()>, tonic::Status> {
        self.config
            .zfs
            .controller()
            .create_volume(&volume.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        return Ok(Response::new(()));
    }

    async fn destroy(&self, name: Request<ZfsName>) -> Result<Response<()>, tonic::Status> {
        self.config
            .zfs
            .controller()
            .destroy(name.get_ref().name.clone())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        return Ok(Response::new(()));
    }
}

#[cfg(test)]
mod tests {
    mod status {
        use crate::testutil::{get_status_client, make_server};

        #[tokio::test]
        async fn test_ping() {
            let mut client = get_status_client(make_server(None).await.unwrap())
                .await
                .unwrap();
            assert!(
                client.ping(tonic::Request::new(())).await.is_ok(),
                "can ping the server"
            );
        }
    }

    #[cfg(feature = "zfs")]
    mod zfs {
        use crate::{
            grpc::ZfsListFilter,
            testutil::{create_zpool, destroy_zpool, get_zfs_client, make_server},
        };

        #[tokio::test]
        async fn test_zfs_operations() {
            let _ = destroy_zpool("default", None);
            let file = create_zpool("default").unwrap();
            let mut client = get_zfs_client(make_server(None).await.unwrap())
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter::default()))
                .await
                .unwrap();

            assert_eq!(res.into_inner().entries.len(), 0);

            destroy_zpool("default", Some(&file)).unwrap();
        }
    }
}
