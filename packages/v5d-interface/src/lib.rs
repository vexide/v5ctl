use std::{io, path::PathBuf};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use socket2::{Domain, SockAddr, Socket, Type};

pub fn socket_path() -> PathBuf {
    dirs_next::runtime_dir()
        .expect("Currently, only Linux is supported by the V5 Daemon")
        .join("v5d.sock")
}

pub fn connect_to_socket() -> io::Result<Socket> {
    let path = socket_path();
    debug!("Connecting to UNIX socket at {:?}", path);

    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    socket.connect(&SockAddr::unix(&path)?)?;

    info!("Connected to UNIX socket at {:?}", path);
    Ok(socket)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Test(String),
}