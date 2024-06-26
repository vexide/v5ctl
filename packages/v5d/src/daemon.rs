use std::io;

use log::{debug, error, info};
use thiserror::Error;
use tokio::{io::AsyncReadExt, net::{UnixListener, UnixStream}};
use v5d_interface::DaemonCommand;
use vex_v5_serial::connection::ConnectionError;

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
    brain_connection: GenericConnection,
}
impl Daemon {
    pub async fn new(connection_type: ConnectionType) -> anyhow::Result<Self> {
        Ok(Self {
            socket: setup_socket()?,
            brain_connection: setup_connection(connection_type).await?,
        })
    }

    pub async fn run(self) {
        loop {
            match self.socket.accept().await {
                Ok((stream, _addr)) => {
                    if let Err(e) = self.handle_connection(stream).await {
                        error!("Failed to handle connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn perform_command(&self, command: DaemonCommand) -> Result<(), DaemonError> {
        match command {
            DaemonCommand::Test(message) => {
                info!("Received test command with message: {}", message);
            }
        }

        Ok(())
    }

    async fn handle_connection(&self, mut stream: UnixStream) -> Result<(), DaemonError> {
        info!("Accepted connection from client");
        let mut content = String::new();
        stream.read_to_string(&mut content).await?;

        let command: DaemonCommand = serde_json::from_str(&content)?;
        debug!("Received command: {:?}", command);
        self.perform_command(command).await?;

        Ok(())
    }
}
