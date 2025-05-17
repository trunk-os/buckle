use crate::grpc::status_client::StatusClient;
use crate::server::Server;
use anyhow::{anyhow, Result};
use std::sync::LazyLock;
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tonic::transport::Channel;

pub(crate) const BUCKLE_TEST_ZPOOL_PREFIX: &str = "buckle-test";
pub(crate) const DEFAULT_CONFIG: LazyLock<crate::config::Config> =
    LazyLock::new(|| crate::config::Config {
        zfs: crate::config::ZFSConfig {
            pool: format!("{}-default", BUCKLE_TEST_ZPOOL_PREFIX),
        },
    });

pub(crate) async fn find_listener() -> Result<SocketAddr> {
    for x in 3000..32767 {
        let addr: SocketAddr = format!("127.0.0.1:{}", x).parse()?;
        match TcpListener::bind(addr).await {
            Ok(_) => return Ok(addr),
            Err(_) => {}
        }
    }

    Err(anyhow!("could not find open port"))
}

pub(crate) async fn make_server(config: Option<crate::config::Config>) -> Result<SocketAddr> {
    let server = Server::new_with_config(Some(config.unwrap_or_else(|| DEFAULT_CONFIG.clone())));
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

    if !std::process::Command::new("truncate")
        .args(vec!["-s", "5G", path.to_str().unwrap()])
        .stdout(std::io::stdout())
        .stderr(std::io::stderr())
        .status()?
        .success()
    {
        return Err(anyhow!("Could not grow file for zpool"));
    }

    let name = format!("{}-{}", BUCKLE_TEST_ZPOOL_PREFIX, name);
    if !std::process::Command::new("zpool")
        .args(vec!["create", &name, path.to_str().unwrap()])
        .stdout(std::io::stdout())
        .stderr(std::io::stderr())
        .status()?
        .success()
    {
        return Err(anyhow!("could not create zpool '{}'", name));
    }

    Ok(path.to_string_lossy().to_string())
}

#[cfg(feature = "zfs")]
pub(crate) fn destroy_zpool(name: &str, file: Option<&str>) -> Result<()> {
    let name = format!("{}-{}", BUCKLE_TEST_ZPOOL_PREFIX, name);
    if !std::process::Command::new("zpool")
        .args(vec!["destroy", "-f", &name])
        .stdout(std::io::stdout())
        .stderr(std::io::stderr())
        .status()?
        .success()
    {
        return Err(anyhow!("could not destroy zpool: {}", name));
    }

    if let Some(file) = file {
        return Ok(std::fs::remove_file(&file)?);
    }

    Ok(())
}

#[cfg(feature = "zfs")]
pub(crate) fn list_zpools() -> Result<Vec<String>> {
    let out = std::process::Command::new("zpool")
        .args(vec!["list"])
        .stderr(std::io::stderr())
        .output()?;
    if out.status.success() {
        let out = String::from_utf8(out.stdout)?;
        let lines = out.split('\n');

        let mut ret = Vec::new();

        for line in lines.skip(1) {
            let mut name = String::new();
            for ch in line.chars() {
                if ch != ' ' {
                    name.push(ch)
                } else {
                    break;
                }
            }
            ret.push(name);
        }

        return Ok(ret);
    }

    Err(anyhow!("error listing zpools"))
}

mod tests {
    #[cfg(feature = "zfs")]
    mod zfs {
        use super::super::{create_zpool, destroy_zpool, list_zpools, BUCKLE_TEST_ZPOOL_PREFIX};

        #[test]
        fn create_remove_zpool() {
            let _ = destroy_zpool("testutil-test", None);
            let file = create_zpool("testutil-test").unwrap();
            assert!(file.len() > 0);
            assert!(list_zpools()
                .unwrap()
                .contains(&format!("{}-testutil-test", BUCKLE_TEST_ZPOOL_PREFIX)));
            destroy_zpool("testutil-test", Some(&file)).unwrap();
            assert!(!std::fs::exists(file).unwrap())
        }
    }
}
