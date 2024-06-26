mod connection;
mod daemon;

use std::io;

use clap::Parser;
use daemon::Daemon;
use log::info;
use tokio::net::UnixListener;
use v5d_interface::socket_path;

#[derive(Debug, Clone, clap::ValueEnum)]
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

/// Creates a UNIX socket to communicate with the V5 Daemon
pub fn setup_socket() -> io::Result<UnixListener> {
    let path = socket_path();

    let socket = UnixListener::bind(&path)?;

    info!("UNIX socket created and bound to {:?}", path);
    info!("Listening for incoming connections...");
    Ok(socket)
}

fn on_shutdown() {
    info!("Shutting down...");
    // Clean up the socket file
    let _ = std::fs::remove_file(socket_path());
    info!("Shutdown complete!");
    std::process::exit(0);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        Default::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    ctrlc::set_handler(on_shutdown)?;

    let daemon = Daemon::new(args.connection_type).await?;
    daemon.run().await;

    Ok(())
}
