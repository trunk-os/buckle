use buckle::server::Server;

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    Server::default().start("[::]:5001".parse()?).await
}
