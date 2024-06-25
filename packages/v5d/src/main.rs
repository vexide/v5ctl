use std::io::Write;
use std::io::{self, Read};

use log::{debug, error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use v5d_interface::{socket_path, DaemonResponse};
use v5d_interface::DaemonCommand;

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
    simplelog::TermLogger::init(
        log::LevelFilter::Trace,
        Default::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    ctrlc::set_handler(on_shutdown)?;

    let socket = setup_socket()?;
    loop {
        match socket.accept().await {
            Ok((mut stream, _addr)) => {
                stream.readable().await?;
                info!("Accepted connection from client");
                let mut content = String::new();
                stream.read_to_string(&mut content).await?;
                
                let command: DaemonCommand = serde_json::from_str(&content)?;
                debug!("Received command: {:?}", command);
                
                stream.writable().await?;
                let DaemonCommand::Test(string) = command;

                let response = DaemonResponse::Test(string);
                stream.write_all(serde_json::to_string(&response)?.as_bytes()).await?;
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
