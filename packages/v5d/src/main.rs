mod connection;
mod daemon;

use clap::Parser;
use daemon::Daemon;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ConnectionType {
    Bluetooth,
    Serial,
    Auto,
}

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(long, short)]
    connection_type: ConnectionType,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .pretty()
        .init();

    let daemon = Daemon::new(args.connection_type).await?;

    let cancel_token = daemon.cancel_token();
    ctrlc::set_handler(move || {
        cancel_token.cancel();
    })?;

    daemon.run().await;

    Ok(())
}
