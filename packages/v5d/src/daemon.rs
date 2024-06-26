use std::{io, sync::Arc};

use log::{debug, error, info};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    spawn,
    sync::Mutex,
};
use v5d_interface::{DaemonCommand, DaemonResponse, ProgramData};
use vex_v5_serial::connection::{Connection, ConnectionError};

use crate::{
    connection::{setup_connection, GenericConnection},
    setup_socket, ConnectionType,
};

#[derive(Debug, Error)]
pub enum DaemonError {
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
    connection_type: ConnectionType,
}
impl Daemon {
    pub async fn new(connection_type: ConnectionType) -> Result<Self, DaemonError> {
        Ok(Self {
            socket: setup_socket()?,
            brain_connection: Mutex::new(setup_connection(connection_type).await?),
            connection_type,
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

    async fn perform_command(
        self: Arc<Self>,
        command: DaemonCommand,
    ) -> Result<Option<DaemonResponse>, DaemonError> {
        let response = match command {
            DaemonCommand::MockTap { x, y } => {
                self.brain_connection
                    .lock()
                    .await
                    .execute_command(vex_v5_serial::commands::screen::MockTap { x, y })
                    .await?;
                Some(DaemonResponse::BasicAck { successful: true })
            }
            DaemonCommand::UploadProgram {
                name,
                description,
                icon,
                slot,
                compression,
                after_upload,
                data,
                program_type,
            } => {
                let command = vex_v5_serial::commands::file::UploadProgram {
                    name,
                    program_type,
                    description,
                    icon,
                    slot,
                    compress_program: compression,
                    after_upload: after_upload.into(),
                    data: data.into(),
                };
                self.brain_connection.lock().await.execute_command(command).await?;

                None
            }
            DaemonCommand::Shutdown => {
                info!("Received shutdown command");
                super::shutdown();
            }
            DaemonCommand::Reconnect => {
                let mut connection = self.brain_connection.lock().await;
                *connection = setup_connection(self.connection_type).await?;
                Some(DaemonResponse::BasicAck { successful: true })
            }
        };

        Ok(response)
    }

    async fn handle_connection(self: Arc<Self>, mut stream: UnixStream) -> Result<(), DaemonError> {
        info!("Accepted connection from client");
        let mut content = String::new();
        stream.read_to_string(&mut content).await?;
        info!("Received content: {}", content);
        stream.read_to_end(&mut Vec::new()).await?;

        let command: DaemonCommand = serde_json::from_str(&content)?;
        debug!("Received command: {:?}", command);
        let response = match self.perform_command(command).await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to perform command: {}", e);
                Some(DaemonResponse::BasicAck { successful: false })
            }
        };
        if let Some(response) = response {
            let content = serde_json::to_string(&response)?;
            stream.writable().await?;
            let content_bytes = content.as_bytes();
            stream.write_all(content_bytes).await?;
        }

        Ok(())
    }
}
