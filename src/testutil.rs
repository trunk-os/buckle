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

#[cfg(feature = "zfs")]
pub(crate) fn create_zpool(name: &str) -> Result<String> {
    std::fs::create_dir_all("tmp")?;

    let (_, path) = tempfile::NamedTempFile::new_in("tmp")?.keep()?;

    std::process::Command::new("truncate")
        .args(vec!["-s", "5G", path.to_str().unwrap()])
        .status()?;

    std::process::Command::new("zpool")
        .args(vec![
            "create",
            &format!("buckle-test-{}", name),
            path.to_str().unwrap(),
        ])
        .stdout(std::io::stdout())
        .status()?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg(feature = "zfs")]
pub(crate) fn destroy_zpool(name: &str, file: &str) -> Result<()> {
    std::process::Command::new("zpool")
        .args(vec!["destroy", "-f", &format!("buckle-test-{}", name)])
        .status()?;
    Ok(std::fs::remove_file(&file)?)
}

#[cfg(test)]
#[cfg(feature = "zfs")]
mod tests {
    use super::{create_zpool, destroy_zpool};
    #[test]
    fn create_remove_zfs() {
        let file = create_zpool("testutil-test").unwrap();
        assert!(file.len() > 0);
        destroy_zpool("testutil-test", &file).unwrap();
        assert!(!std::fs::exists(file).unwrap())
    }
}
