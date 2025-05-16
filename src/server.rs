use std::net::SocketAddr;

use crate::grpc::status_server::{Status, StatusServer};
use tonic::{transport::Server as TransportServer, Request, Response, Result};

// FIXME needs a way to shut down
#[derive(Debug, Default, Clone)]
pub struct Server;

impl Server {
    pub fn start(
        &self,
        addr: SocketAddr,
    ) -> impl std::future::Future<Output = Result<(), tonic::transport::Error>> {
        TransportServer::builder()
            .add_service(StatusServer::new(self.clone()))
            .serve(addr)
    }
}

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<()>, tonic::Status> {
        return Ok(Response::new(()));
    }
}

#[cfg(test)]
mod tests {
    mod status {
        use crate::testutil::{get_status_client, make_server};

        #[tokio::test]
        async fn test_ping() {
            let mut client = get_status_client(make_server().await.unwrap())
                .await
                .unwrap();
            assert!(
                client.ping(tonic::Request::new(())).await.is_ok(),
                "can ping the server"
            );
        }
    }
}
