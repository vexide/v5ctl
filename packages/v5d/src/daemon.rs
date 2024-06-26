use std::{io, sync::Arc};

use log::{debug, error, info};
use thiserror::Error;
use tokio::{
    io::AsyncReadExt, net::{UnixListener, UnixStream}, spawn, sync::{Mutex, RwLock}
};
use v5d_interface::DaemonCommand;
use vex_v5_serial::connection::{Connection, ConnectionError};

use crate::{
    connection::{setup_connection, GenericConnection},
    setup_socket, ConnectionType,
};

#[derive(Debug, Error)]
enum DaemonError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    #[error("Communication serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

pub struct Daemon {
    socket: UnixListener,
    brain_connection: Mutex<GenericConnection>,
}
impl Daemon {
    pub async fn new(connection_type: ConnectionType) -> anyhow::Result<Self> {
        Ok(Self {
            socket: setup_socket()?,
            brain_connection: Mutex::new(setup_connection(connection_type).await?),
        })
    }

    pub async fn run(self) {
        let this = Arc::new(self);
        loop {
            match this.socket.accept().await {
                Ok((stream, _addr)) => {
                    let this = this.clone();
                    spawn(async move {
                        if let Err(e) = this.handle_connection(stream).await {
                            error!("Failed to handle connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn perform_command(self: Arc<Self>, command: DaemonCommand) -> Result<(), DaemonError> {
        match command {
            DaemonCommand::MockTap { x, y } => {
                self.brain_connection
                    .lock()
                    .await
                    .execute_command(vex_v5_serial::commands::screen::MockTap { x, y })
                    .await?;
            }
            DaemonCommand::Shutdown => {
                info!("Received shutdown command");
                super::on_shutdown();
            }
        }

        Ok(())
    }

    async fn handle_connection(self: Arc<Self>, mut stream: UnixStream) -> Result<(), DaemonError> {
        info!("Accepted connection from client");
        let mut content = String::new();
        stream.read_to_string(&mut content).await?;

        let command: DaemonCommand = serde_json::from_str(&content)?;
        debug!("Received command: {:?}", command);
        self.perform_command(command).await?;

        Ok(())
    }
}
