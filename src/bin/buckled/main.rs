use buckle::{config::Config, server::Server};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let config = if std::env::args().len() != 1 {
        Config::from_file(std::env::args().skip(1).next().unwrap().into())?
    } else {
        Config::default()
    };

    Ok(Server::new_with_config(Some(config)).start()?.await?)
}
