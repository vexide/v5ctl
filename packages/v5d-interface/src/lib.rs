use std::{io, path::PathBuf};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use socket2::{Domain, SockAddr, Socket, Type};
use tokio::{io::{AsyncReadExt, Interest}, net::UnixStream};

pub fn socket_path() -> PathBuf {
    dirs_next::runtime_dir()
        .expect("Currently, only Linux is supported by the V5 Daemon")
        .join("v5d.sock")
}

pub async fn connect_to_socket() -> io::Result<UnixStream> {
    let path = socket_path();
    debug!("Connecting to UNIX socket at {:?}", path);

    let socket = UnixStream::connect(&path).await?;
    socket.ready(Interest::READABLE | Interest::WRITABLE).await?;

    info!("Connected to UNIX socket at {:?}", path);
    Ok(socket)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    Test(String),
}
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    Test(String),
}
