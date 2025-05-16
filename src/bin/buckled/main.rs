use buckle::grpc::status_server::StatusServer;
use buckle::server::Server as BuckleServer;
use tonic::transport::Server;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::builder()
        .add_service(StatusServer::new(BuckleServer))
        .serve("[::]:5001".parse()?)
        .await?;
    Ok(())
}
