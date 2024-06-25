use std::io::{self, Read};

use log::{debug, error, info};
use socket2::{Domain, SockAddr, Socket, Type};
use v5d_interface::socket_path;
use v5d_interface::Command;

/// Creates a UNIX socket to communicate with the V5 Daemon
pub fn setup_socket() -> io::Result<Socket> {
    let path = socket_path();

    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    socket.bind(&SockAddr::unix(&path)?)?;
    socket.listen(128)?;

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
        log::LevelFilter::Debug,
        Default::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    ctrlc::set_handler(on_shutdown)?;

    let socket = setup_socket()?;
    loop {
        match socket.accept() {
            Ok((mut stream, _addr)) => {
                info!("Accepted connection from client");
                let mut content = String::new();
                stream.read_to_string(&mut content)?;
                
                let command: Command = serde_json::from_str(&content)?;
                debug!("Received command: {:?}", command);
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
