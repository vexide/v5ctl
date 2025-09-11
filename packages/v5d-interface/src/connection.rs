//! Delegation of [`DeviceInterface`] calls to another process.
//!
//! This module contains utilities for either both delegating requests
//! regarding devices to another process or configuring your own
//! process to accept those requests using an implementation of
//! [`DeviceInterface`].
//!
//! This is implemented using OS inter-process communication APIs under
//! the hood, but this is all abstracted away using [`DaemonConnection`]
//! (client) and [`DaemonListener`] (server).

use std::{fmt::Debug, io::ErrorKind, sync::Arc};

use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, Name, ToNsName,
    tokio::{Listener, RecvHalf, SendHalf, Stream, prelude::*},
};
use serde::{Deserialize, Serialize};
use snafu::{IntoError, ResultExt, Snafu};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    runtime::Handle,
    sync::Mutex,
    task::block_in_place,
};
use tracing::{debug, error, info, trace};

fn get_socket_name() -> Name<'static> {
    "vexide-v5d.sock"
        .to_ns_name::<GenericNamespaced>()
        .expect("socket name should be valid")
}

struct BufStream {
    reader: BufReader<RecvHalf>,
    writer: BufWriter<SendHalf>,
}

impl BufStream {
    fn new(stream: Stream) -> Self {
        let (reader, writer) = stream.split();
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }
}

/// A server that listens for device communication requests.
///
/// This struct allows you to expose your own [`DeviceInterface`]
/// implementation so that other processes can access it using
/// a [`DaemonConnection`] struct.
pub struct DaemonListener {
    listener: Listener,
}

impl DaemonListener<I> {
    /// Register this process as the current device daemon.
    ///
    /// Next, call [`Self::handle_connections`] to begin handling
    /// requests.
    pub fn new(interface: I) -> Result<Self, ConnectionError> {
        let listener = ListenerOptions::new()
            .name(get_socket_name())
            .create_tokio()
            .map_err(|err| {
                if err.kind() == ErrorKind::AddrInUse {
                    ExistingServerSnafu.into_error(err)
                } else {
                    err.into()
                }
            })?;

        Ok(Self {
            interface: Arc::new(Mutex::new(interface)),
            listener,
        })
    }

    pub fn interface(&self) -> Arc<Mutex<I>> {
        self.interface.clone()
    }

    /// Begin handling incoming connections.
    pub async fn handle_connections(&mut self) {
        loop {
            let stream = match self.listener.accept().await {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "There was an error with an incoming connection");
                    continue;
                }
            };

            let interface = self.interface();

            tokio::spawn(async move {
                let mut connection = IncomingConnection::new(stream, interface);
                if let Err(e) = connection.handle_commands().await {
                    error!(error = %e, "An error occurred while handling a connection's command");
                }
            });
        }
    }
}

/// Represents a single IPC connection.
struct IncomingConnection<I: DeviceInterface + Send> {
    stream: BufStream,
    interface: Arc<Mutex<I>>,
}

impl<I: DeviceInterface + Send> IncomingConnection<I> {
    pub fn new(stream: Stream, interface: Arc<Mutex<I>>) -> Self {
        Self {
            stream: BufStream::new(stream),
            interface,
        }
    }

    /// Handle incoming commands until the connection is closed.
    async fn handle_commands(&mut self) -> Result {
        info!("Accepted connection from client");

        while let Some(command) = self.read().await? {
            self.dispatch_command(command).await?;
        }

        Ok(())
    }

    /// Reads the next message, or returns `None` if the other
    /// process disconnected.
    async fn read(&mut self) -> Result<Option<DaemonCommand>> {
        let mut command_string = String::new();
        let size = self.stream.reader.read_line(&mut command_string).await?;
        if size == 0 {
            return Ok(None);
        }

        trace!(?command_string, "Received serialized command");

        let command: DaemonCommand = serde_json::from_str(&command_string)?;
        debug!(?command, "Received command");

        Ok(Some(command))
    }

    async fn reply<T: Serialize + Debug>(&mut self, response: &T) -> Result {
        debug!(?response, "Replying to request");
        let mut serialized_response = serde_json::to_string(&response)?;
        serialized_response.push('\n');

        trace!(?serialized_response, "Sending serialized response");
        self.stream
            .writer
            .write_all(serialized_response.as_bytes())
            .await?;

        Ok(())
    }

    /// Executes a serialized command using the [`DeviceInterface`] stored
    /// in this struct.
    async fn dispatch_command(&mut self, command: DaemonCommand) -> Result {
        let interface = self.interface.clone();
        let mut interface = interface.lock().await;

        let result: CompletionResponse = match command {
            DaemonCommand::MockTap { x, y } => interface.mock_tap(x, y).await,
            DaemonCommand::UploadProgram(opts) => {
                let res = interface
                    .upload_program(opts, |progress| {
                        block_in_place(|| {
                            Handle::current().block_on(async {
                                _ = self
                                    .reply(&TransferProgressResponse::Progress(progress))
                                    .await;
                            })
                        })
                    })
                    .await
                    .map_err(RemoteError::from);

                self.reply(&TransferProgressResponse::Complete(res)).await?;
                return Ok(());
            }
            DaemonCommand::PairingPin(pin) => interface.pairing_pin(pin).await,
            DaemonCommand::Reconnect => interface.reconnect().await,
            DaemonCommand::RequestPair => interface.request_pair().await,
            DaemonCommand::Shutdown => interface.shutdown().await,
        }
        .map_err(RemoteError::from);

        self.reply(&result).await?;

        Ok(())
    }
}
