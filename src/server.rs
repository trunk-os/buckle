use std::net::SocketAddr;

use crate::grpc::status_server::{Status, StatusServer};
use tonic::{transport::Server as TransportServer, Request, Response, Result};

#[derive(Debug, Default, Clone)]
pub struct Server;

impl Server {
    pub async fn start(&self, addr: SocketAddr) -> Result<(), anyhow::Error> {
        TransportServer::builder()
            .add_service(StatusServer::new(self.clone()))
            .serve(addr)
            .await?;

        Ok(())
    }
}

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<()>, tonic::Status> {
        return Ok(Response::new(()));
    }
}
