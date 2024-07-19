use std::{io, sync::Arc};

use log::{debug, error, info, trace};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    spawn,
    sync::{mpsc::Sender, Mutex},
};
use v5d_interface::{DaemonCommand, DaemonResponse, UploadStep};
use vex_v5_serial::connection::{
    generic::{GenericConnection, GenericError},
    Connection,
};

use crate::{connection::setup_connection, setup_socket, ConnectionType};

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Connection error: {0}")]
    Connection(#[from] GenericError),
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
                        if let Err(e) = this.handle_connection(BufReader::new(stream)).await {
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
        stream: Arc<Mutex<BufReader<UnixStream>>>,
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
                let (response_sender, mut response_receiver) =
                    tokio::sync::mpsc::channel::<DaemonResponse>(1000);
                let response_sender = Arc::new(Mutex::new(response_sender));

                spawn(async move {
                    let mut stream = stream.lock().await;
                    while let Some(response) = response_receiver.recv().await {
                        let mut content = serde_json::to_string(&response).unwrap();
                        content.push('\n');
                        let content_bytes = content.as_bytes();
                        stream.write_all(content_bytes).await.unwrap();
                        stream.flush().await.unwrap();
                    }
                });

                fn generate_callback(
                    step: UploadStep,
                    sender: Arc<Mutex<Sender<DaemonResponse>>>,
                ) -> Box<dyn FnMut(f32) + Send> {
                    Box::new(move |percent| {
                        let sender = sender.clone();
                        tokio::task::block_in_place(move || {
                            let response = DaemonResponse::TransferProgress { percent, step };
                            let sender = sender.blocking_lock();
                            trace!("CALLBACK: {:?}", response);
                            sender.blocking_send(response).unwrap();
                        });
                    })
                }

                let command = vex_v5_serial::commands::file::UploadProgram {
                    name,
                    program_type,
                    description,
                    icon,
                    slot: slot - 1,
                    compress_program: compression,
                    after_upload: after_upload.into(),
                    data,
                    ini_callback: Some(generate_callback(UploadStep::Ini, response_sender.clone())),
                    monolith_callback: Some(generate_callback(
                        UploadStep::Monolith,
                        response_sender.clone(),
                    )),
                    cold_callback: Some(generate_callback(
                        UploadStep::Cold,
                        response_sender.clone(),
                    )),
                    hot_callback: Some(generate_callback(UploadStep::Hot, response_sender.clone())),
                };

                Some(DaemonResponse::TransferComplete(
                    match self
                        .brain_connection
                        .lock()
                        .await
                        .execute_command(command)
                        .await
                    {
                        Ok(_) => Ok(()),
                        Err(err) => Err(format!("Failed to upload program: {}", err)),
                    },
                ))
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
            DaemonCommand::RequestPair => {
                let mut connection = self.brain_connection.lock().await;
                Some(match *connection {
                    GenericConnection::Bluetooth(ref mut connection) => {
                        connection
                            .request_pairing()
                            .await
                            .map_err(Into::<GenericError>::into)?;
                        DaemonResponse::BasicAck { successful: true }
                    }
                    GenericConnection::Serial(_) => DaemonResponse::BasicAck { successful: false },
                })
            }
            DaemonCommand::PairingPin(pin) => {
                let mut connection = self.brain_connection.lock().await;
                Some(match *connection {
                    GenericConnection::Bluetooth(ref mut connection) => {
                        connection
                            .authenticate_pairing(pin)
                            .await
                            .map_err(Into::<GenericError>::into)?;
                        DaemonResponse::BasicAck { successful: true }
                    }
                    GenericConnection::Serial(_) => DaemonResponse::BasicAck { successful: false },
                })
            }
        };

        Ok(response)
    }

    async fn handle_connection(
        self: Arc<Self>,
        mut stream: BufReader<UnixStream>,
    ) -> Result<(), DaemonError> {
        info!("Accepted connection from client");
        let mut content = String::new();
        stream.read_line(&mut content).await?;

        let stream = Arc::new(Mutex::new(stream));
        let command: DaemonCommand = serde_json::from_str(&content)?;
        debug!("Received command: {:?}", command);
        let response = match self.perform_command(command, stream.clone()).await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to perform command: {}", e);
                Some(DaemonResponse::BasicAck { successful: false })
            }
        };
        if let Some(response) = response {
            let mut content = serde_json::to_string(&response)?;
            content.push('\n');
            let content_bytes = content.as_bytes();
            stream.lock().await.write_all(content_bytes).await?;
        }

        Ok(())
    }
}
