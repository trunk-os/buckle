use std::{fs::Permissions, os::unix::fs::PermissionsExt};

use crate::{
    grpc::{
        status_server::{Status, StatusServer},
        zfs_server::{Zfs, ZfsServer},
        PingResult, ZfsDataset, ZfsList, ZfsListFilter, ZfsModifyDataset, ZfsModifyVolume, ZfsName,
        ZfsVolume,
    },
    sysinfo::Info,
};
use tonic::{transport::Server as TransportServer, Request, Response, Result};
use tonic_middleware::MiddlewareLayer;
use tracing::info;

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
        info!("Starting service.");

        if let Some(parent) = self.config.socket.to_path_buf().parent() {
            std::fs::create_dir_all(&parent)?;
        }

        if std::fs::exists(&self.config.socket)? {
            std::fs::remove_file(&self.config.socket)?;
        }

        let uds = tokio::net::UnixListener::bind(&self.config.socket)?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

        std::fs::set_permissions(&self.config.socket, Permissions::from_mode(0o600))?;

        Ok(TransportServer::builder()
            .layer(MiddlewareLayer::new(crate::middleware::LogMiddleware))
            .add_service(StatusServer::new(self.clone()))
            .add_service(ZfsServer::new(self.clone()))
            .serve_with_incoming(uds_stream))
    }
}

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<PingResult>> {
        Ok(Response::new(PingResult {
            info: Some(Info::default().into()),
        }))
    }
}

#[tonic::async_trait]
impl Zfs for Server {
    async fn modify_dataset(&self, info: Request<ZfsModifyDataset>) -> Result<Response<()>> {
        self.config
            .zfs
            .controller()
            .modify_dataset(info.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        Ok(Response::new(()))
    }

    async fn modify_volume(&self, info: Request<ZfsModifyVolume>) -> Result<Response<()>> {
        self.config
            .zfs
            .controller()
            .modify_volume(info.into_inner().into())
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
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
    #[cfg(feature = "zfs")]
    mod status {
        use crate::testutil::{get_status_client, make_server};

        #[tokio::test]
        async fn test_ping() {
            let mut client = get_status_client(make_server(None).await.unwrap())
                .await
                .unwrap();
            let results = client
                .ping(tonic::Request::new(()))
                .await
                .unwrap()
                .into_inner();
            assert!(results.info.is_some());
            let info = results.info.unwrap();
            assert_ne!(info.uptime, 0);
            assert_ne!(info.available_memory, 0);
            assert_ne!(info.total_memory, 0);
            assert_ne!(info.cpus, 0);
            assert_ne!(info.cpu_usage, 0.0);
            assert!(!info.host_name.is_empty());
            assert!(!info.kernel_version.is_empty());
            assert_ne!(info.load_average, [0.0, 0.0, 0.0]);
            assert_ne!(info.processes, 0);
        }
    }

    #[cfg(feature = "zfs")]
    mod zfs {
        use crate::{
            grpc::{
                ZfsDataset, ZfsListFilter, ZfsModifyDataset, ZfsModifyVolume, ZfsName, ZfsType,
                ZfsVolume,
            },
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

            client
                .modify_dataset(tonic::Request::new(ZfsModifyDataset {
                    name: "dataset".into(),
                    modifications: Some(ZfsDataset {
                        name: "dataset2".into(),
                        quota: Some(5 * 1024 * 1024),
                    }),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("dataset2".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 1);

            let item = &res[0];

            assert_eq!(item.kind(), ZfsType::Dataset);
            assert_eq!(item.name, "dataset2");
            assert_eq!(
                item.full_name,
                format!("{}-default/dataset2", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(item.size, 0);
            assert_ne!(item.used, 0);
            assert_ne!(item.refer, 0);
            assert_ne!(item.avail, 0);
            assert_eq!(
                item.mountpoint,
                Some(format!("/{}-default/dataset2", BUCKLE_TEST_ZPOOL_PREFIX))
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
                .modify_volume(tonic::Request::new(ZfsModifyVolume {
                    name: "volume".into(),
                    modifications: Some(ZfsVolume {
                        name: "volume2".into(),
                        size: 5 * 1024 * 1024,
                    }),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("volume2".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 1);

            let item = &res[0];

            assert_eq!(item.kind(), ZfsType::Volume);
            assert_eq!(item.name, "volume2");
            assert_eq!(
                item.full_name,
                format!("{}-default/volume2", BUCKLE_TEST_ZPOOL_PREFIX),
            );
            assert_ne!(item.size, 0);
            assert!(
                item.size < 6 * 1024 * 1024 && item.size > 4 * 1024 * 1024,
                "{}",
                item.size
            );
            assert_ne!(item.used, 0);
            assert_ne!(item.refer, 0);
            assert_ne!(item.avail, 0);
            assert_eq!(item.mountpoint, None);

            client
                .destroy(tonic::Request::new(ZfsName {
                    name: "volume2".to_string(),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("volume2".to_string()),
                }))
                .await
                .unwrap()
                .into_inner()
                .entries;

            assert_eq!(res.len(), 0);

            client
                .destroy(tonic::Request::new(ZfsName {
                    name: "dataset2".to_string(),
                }))
                .await
                .unwrap();

            let res = client
                .list(tonic::Request::new(ZfsListFilter {
                    filter: Some("dataset2".to_string()),
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
