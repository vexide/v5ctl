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

use self::format::{DaemonCommand, TransferProgressResponse};
use crate::{
    DeviceInterface, TransferProgress, UploadProgramOpts,
    connection::format::CompletionResponse,
    error::{RemoteError, Result, SerializeError, SerializeSnafu},
};

mod format;

#[derive(Debug, Snafu)]
pub enum ConnectionError {
    #[snafu(transparent)]
    SerializeMsg { source: SerializeError },
    #[snafu(transparent)]
    Remote { source: RemoteError },
    #[snafu(transparent)]
    Io { source: io::Error },
    #[snafu(display("Cannot listen for connections because another v5d server is running"))]
    ExistingServer { source: io::Error },
}

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

/// A connection to a remote VEX device connection implementation.
///
/// This struct implements [`DeviceInterface`] by asking another
/// process to do the work for it.
pub struct DaemonConnection {
    stream: BufStream,
}

impl DaemonConnection {
    /// Connect to a running process exposing its device
    pub async fn new() -> Result<Self, ConnectionError> {
        let stream = Stream::connect(get_socket_name()).await?;

        Ok(Self {
            stream: BufStream::new(stream),
        })
    }

    pub(crate) async fn send(
        &mut self,
        cmd: &DaemonCommand,
        wait: bool,
    ) -> Result<(), ConnectionError> {
        let mut content =
            serde_json::to_vec(&cmd).context(SerializeSnafu { deserialize: false })?;
        content.push(b'\n');
        self.stream.writer.write_all(&content).await?;

        if wait {
            self.wait_for_ack().await?;
        }

        Ok(())
    }

    pub(crate) async fn recv<T: for<'a> Deserialize<'a>>(&mut self) -> Result<T, ConnectionError> {
        let mut response = String::new();
        self.stream.reader.read_line(&mut response).await?;
        let responses =
            serde_json::from_str(&response).context(SerializeSnafu { deserialize: true })?;
        Ok(responses)
    }

    pub(crate) async fn wait_for_ack(&mut self) -> Result<(), ConnectionError> {
        self.recv::<CompletionResponse>().await??;
        Ok(())
    }
}

impl DeviceInterface for DaemonConnection {
    async fn mock_tap(&mut self, x: u16, y: u16) -> Result {
        self.send(&DaemonCommand::MockTap { x, y }, true).await?;
        Ok(())
    }

    async fn upload_program(
        &mut self,
        opts: UploadProgramOpts,
        mut handle_progress: impl FnMut(TransferProgress) + Send,
    ) -> Result {
        self.send(&DaemonCommand::UploadProgram(opts), false)
            .await?;

        loop {
            let msg = self.recv().await?;

            match msg {
                TransferProgressResponse::Progress(progress) => {
                    handle_progress(progress);
                }
                TransferProgressResponse::Complete(response) => {
                    return Ok(response?);
                }
            }
        }
    }

    async fn shutdown(&mut self) -> Result {
        self.send(&DaemonCommand::Shutdown, true).await?;
        self.wait_for_ack().await?;
        Ok(())
    }

    async fn pairing_pin(&mut self, pin: [u8; 4]) -> Result {
        self.send(&DaemonCommand::PairingPin(pin), true).await?;
        Ok(())
    }

    async fn reconnect(&mut self) -> Result {
        self.send(&DaemonCommand::Reconnect, true).await?;
        Ok(())
    }

    async fn request_pair(&mut self) -> Result {
        self.send(&DaemonCommand::RequestPair, true).await?;
        Ok(())
    }
}

/// A server that listens for device communication requests.
///
/// This struct allows you to expose your own [`DeviceInterface`]
/// implementation so that other processes can access it using
/// a [`DaemonConnection`] struct.
pub struct DaemonListener<I: DeviceInterface + Send + 'static> {
    interface: Arc<Mutex<I>>,
    listener: Listener,
}

impl<I: DeviceInterface + Send + 'static> DaemonListener<I> {
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
