use anyhow::Result;
use buckle::grpc::status_client;
use clap::{Parser, Subcommand};
use fancy_duration::AsFancyDuration;

#[derive(Parser, Debug, Clone)]
#[command(version, about="CLI interface to the Control Plane for Trunk", long_about=None)]
struct MainArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Ping,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = MainArgs::parse();

    match args.command {
        Commands::Ping => {
            let mut client = status_client::StatusClient::connect("http://[::]:5001").await?;
            let start = std::time::Instant::now();
            client.ping(tonic::Request::new(())).await?;
            println!(
                "Ping succeded. Latency: {}",
                (std::time::Instant::now() - start).fancy_duration()
            );
        }
    }

    Ok(())
}
