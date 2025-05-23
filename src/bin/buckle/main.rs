use anyhow::Result;
use buckle::client::Client;
use clap::{Parser, Subcommand};
use fancy_duration::AsFancyDuration;

#[derive(Parser, Debug, Clone)]
#[command(version, about="CLI interface to the Control Plane for Trunk", long_about=None)]
struct MainArgs {
    #[arg(
        short = 's',
        help = "The path to the buckle socket",
        default_value = "/trunk/socket/buckled.sock"
    )]
    socket_path: std::path::PathBuf,
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
            let client = Client::new(args.socket_path)?;
            let start = std::time::Instant::now();
            client.status().await?.ping().await?;
            println!(
                "Ping succeded. Latency: {}",
                (std::time::Instant::now() - start).fancy_duration()
            );
        }
    }

    Ok(())
}
