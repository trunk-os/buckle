use buckle::server::Server;

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    Ok(Server::default().start("[::]:5001".parse()?).await?)
}
