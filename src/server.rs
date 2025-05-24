use crate::grpc::{
    status_server::{Status, StatusServer},
    zfs_server::{Zfs, ZfsServer},
    ZfsDataset, ZfsList, ZfsListFilter, ZfsModifyDataset, ZfsModifyVolume, ZfsName, ZfsVolume,
};
use tonic::{transport::Server as TransportServer, Request, Response, Result};

// FIXME needs a way to shut down
#[derive(Debug, Default, Clone)]
pub struct Server {
    config: crate::config::Config,
}

impl Server {
    pub fn new_with_config(config: Option<crate::config::Config>) -> Self {
        match config {
            Some(config) => Self { config },
            None => Self::default(),
        }
    }

    pub fn start(
        &self,
    ) -> anyhow::Result<impl std::future::Future<Output = Result<(), tonic::transport::Error>>>
    {
        let uds = tokio::net::UnixListener::bind(self.config.socket.clone())?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);
        Ok(TransportServer::builder()
            .add_service(StatusServer::new(self.clone()))
            .add_service(ZfsServer::new(self.clone()))
            .serve_with_incoming(uds_stream))
    }
}

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<()>> {
        return Ok(Response::new(()));
    }
}

#[tonic::async_trait]
impl Zfs for Server {
    async fn modify_dataset(&self, _info: Request<ZfsModifyDataset>) -> Result<Response<()>> {
        Ok(Response::new(()))
    }

    async fn modify_volume(&self, _info: Request<ZfsModifyVolume>) -> Result<Response<()>> {
        Ok(Response::new(()))
    }

    async fn list(&self, filter: Request<ZfsListFilter>) -> Result<Response<ZfsList>> {
        let list = self
            .config
            .zfs
            .controller()
            .list(filter.get_ref().filter.clone())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        return Ok(Response::new(list.into()));
    }

    async fn create_dataset(&self, dataset: Request<ZfsDataset>) -> Result<Response<()>> {
        self.config
            .zfs
            .controller()
            .create_dataset(&dataset.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;

        return Ok(Response::new(()));
    }

    async fn create_volume(&self, volume: Request<ZfsVolume>) -> Result<Response<()>> {
        self.config
            .zfs
            .controller()
            .create_volume(&volume.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        return Ok(Response::new(()));
    }

    async fn destroy(&self, name: Request<ZfsName>) -> Result<Response<()>> {
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
            grpc::{ZfsDataset, ZfsListFilter, ZfsName, ZfsType, ZfsVolume},
            testutil::{
                create_zpool, destroy_zpool, get_zfs_client, make_server, BUCKLE_TEST_ZPOOL_PREFIX,
            },
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

            client
                .create_dataset(tonic::Request::new(
                    ZfsDataset {
                        name: "dataset".to_string(),
                        ..Default::default()
                    }
                    .into(),
                ))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter::default()))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 1);

            let item = &res[0];

            assert_eq!(item.kind(), ZfsType::Dataset);
            assert_eq!(item.name, "dataset");
            assert_eq!(
                item.full_name,
                format!("{}-default/dataset", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(item.size, 0);
            assert_ne!(item.used, 0);
            assert_ne!(item.refer, 0);
            assert_ne!(item.avail, 0);
            assert_eq!(
                item.mountpoint,
                Some(format!("/{}-default/dataset", BUCKLE_TEST_ZPOOL_PREFIX))
            );

            client
                .create_volume(tonic::Request::new(
                    ZfsVolume {
                        name: "volume".to_string(),
                        size: 100 * 1024 * 1024,
                    }
                    .into(),
                ))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter::default()))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 2);

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("dataset".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 1);

            let item = &res[0];

            assert_eq!(item.kind(), ZfsType::Dataset);
            assert_eq!(item.name, "dataset");
            assert_eq!(
                item.full_name,
                format!("{}-default/dataset", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(item.size, 0);
            assert_ne!(item.used, 0);
            assert_ne!(item.refer, 0);
            assert_ne!(item.avail, 0);
            assert_eq!(
                item.mountpoint,
                Some(format!("/{}-default/dataset", BUCKLE_TEST_ZPOOL_PREFIX))
            );

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("volume".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 1);

            let item = &res[0];

            assert_eq!(item.kind(), ZfsType::Volume);
            assert_eq!(item.name, "volume");
            assert_eq!(
                item.full_name,
                format!("{}-default/volume", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(item.size, 0);
            assert_ne!(item.used, 0);
            assert_ne!(item.refer, 0);
            assert_ne!(item.avail, 0);
            assert_eq!(item.mountpoint, None);

            client
                .destroy(tonic::Request::new(ZfsName {
                    name: "volume".to_string(),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("volume".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 0);

            client
                .destroy(tonic::Request::new(ZfsName {
                    name: "dataset".to_string(),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("dataset".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 0);

            let res = client
                .list(tonic::Request::new(ZfsListFilter::default()))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 0);

            destroy_zpool("default", Some(&file)).unwrap();
        }
    }
}
