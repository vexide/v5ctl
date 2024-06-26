use std::{io, path::PathBuf};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;

pub fn socket_path() -> PathBuf {
    dirs_next::runtime_dir()
        .expect("Currently, only Linux is supported by the V5 Daemon")
        .join("v5d.sock")
}

pub async fn connect_to_socket() -> io::Result<UnixStream> {
    let path = socket_path();
    debug!("Connecting to UNIX socket at {:?}", path);

    let socket = UnixStream::connect(&path).await?;

    info!("Connected to UNIX socket at {:?}", path);
    Ok(socket)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    MockTap { x: u16, y: u16 },
    Shutdown,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    Test(String),
}
