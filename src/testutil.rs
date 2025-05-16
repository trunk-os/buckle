use crate::grpc::status_client::StatusClient;
use crate::server::Server;
use anyhow::{anyhow, Result};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tonic::transport::Channel;

pub(crate) async fn find_listener() -> Result<SocketAddr> {
    for x in 3000..32767 {
        let addr: SocketAddr = format!("0.0.0.0:{}", x).parse()?;
        match TcpListener::bind(addr).await {
            Ok(_) => return Ok(addr),
            Err(_) => {}
        }
    }

    Err(anyhow!("could not find open port"))
}

pub(crate) async fn make_server() -> Result<SocketAddr> {
    let server = Server::default();
    let addr = find_listener().await?;

    tokio::spawn(async move { server.start(addr.clone()).await.unwrap() });

    // wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(addr)
}

pub(crate) async fn get_status_client(addr: SocketAddr) -> Result<StatusClient<Channel>> {
    Ok(StatusClient::connect(format!("http://{}", addr)).await?)
}
