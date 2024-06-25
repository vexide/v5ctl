use std::io;

use log::{debug, error, info};
use socket2::{Domain, SockAddr, Socket, Type};
use v5d_interface::socket_path;


/// Creates a UNIX socket to communicate with the V5 Daemon
pub fn setup_socket() -> io::Result<Socket> {
    let path = socket_path();
    debug!("Creating UNIX socket at {:?}", path);

    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    socket.bind(&SockAddr::unix(&path)?)?;

    info!("UNIX socket created and bound to {:?}", path);
    info!("Listening for incoming connections...");
    Ok(socket)
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let socket = setup_socket()?;
    loop {
        match socket.accept() {
            Ok((stream, _addr)) => {
                info!("Accepted connection from client");
                // Handle the connection
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
