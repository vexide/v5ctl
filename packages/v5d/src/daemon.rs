use std::{io, sync::Arc};

use log::{debug, error, info, trace};
use snafu::Snafu;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    spawn,
    sync::{Mutex, MutexGuard, mpsc::Sender},
};
use tokio_util::sync::CancellationToken;
use v5d_interface::{
    DeviceInterface, TransferProgress, UploadProgramOpts, UploadStep, connection::DaemonListener,
};
use vex_v5_serial::{
    commands::screen::MockTap,
    connection::{
        Connection,
        bluetooth::BluetoothConnection,
        generic::{GenericConnection, GenericError},
    },
};

use crate::{ConnectionType, connection::setup_connection};

#[derive(Debug, Snafu)]
pub enum DaemonError {
    #[snafu(transparent)]
    Connection { source: GenericError },
    #[snafu(display("Failed to serialize message"))]
    Serde { source: serde_json::Error },
    #[snafu(transparent)]
    Io { source: io::Error },
    #[snafu(display(
        "This operation may only be performed on {required:?} connections (have: {actual:?} connection)"
    ))]
    WrongConnectionType {
        required: vex_v5_serial::connection::ConnectionType,
        actual: vex_v5_serial::connection::ConnectionType,
    },
}

pub struct Daemon {
    brain_connection: Mutex<GenericConnection>,
    connection_type: ConnectionType,
    cancel_token: CancellationToken,
}
impl Daemon {
    pub async fn new(connection_type: ConnectionType) -> Result<Self, DaemonError> {
        Ok(Self {
            brain_connection: Mutex::new(setup_connection(connection_type).await?),
            connection_type,
            cancel_token: CancellationToken::new(),
        })
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub async fn run(self) {
        let token = self.cancel_token.clone();
        let mut listener = DaemonListener::new(self).expect("socket should be available");
        tokio::select! {
            _ = token.cancelled() => {}
            _ = listener.handle_connections() => {}
        }
    }

    fn get_bluetooth(&mut self) -> Result<&mut BluetoothConnection, DaemonError> {
        let connection = self.brain_connection.get_mut();

        if let GenericConnection::Bluetooth(connection) = connection {
            Ok(connection)
        } else {
            WrongConnectionTypeSnafu {
                actual: connection.connection_type(),
                required: vex_v5_serial::connection::ConnectionType::Bluetooth,
            }
            .fail()?
        }
    }
}

impl DeviceInterface for Daemon {
    async fn mock_tap(&mut self, x: u16, y: u16) -> v5d_interface::Result {
        let conn = self.brain_connection.get_mut();
        conn.execute_command(MockTap { x, y }).await?;
        Ok(())
    }

    async fn upload_program(
        &mut self,
        opts: UploadProgramOpts,
        handle_progress: impl FnMut(TransferProgress) + Send,
    ) -> v5d_interface::Result {
        let reporter = Arc::new(Mutex::new(handle_progress));

        fn progress_callback_for<'a>(
            step: UploadStep,
            reporter: Arc<Mutex<impl FnMut(TransferProgress) + Send + 'a>>,
        ) -> Box<dyn FnMut(f32) + Send + 'a> {
            Box::new(move |percent| {
                tokio::task::block_in_place(|| {
                    reporter.blocking_lock()(TransferProgress { percent, step });
                });
            })
        }

        let command = vex_v5_serial::commands::file::UploadProgram {
            name: opts.name,
            program_type: opts.program_type,
            description: opts.description,
            icon: opts.icon,
            slot: opts.slot - 1,
            compress_program: opts.compression,
            after_upload: opts.after_upload.into(),
            data: opts.data,
            ini_callback: Some(progress_callback_for(UploadStep::Ini, reporter.clone())),
            bin_callback: Some(progress_callback_for(UploadStep::Bin, reporter.clone())),
            lib_callback: Some(progress_callback_for(UploadStep::Lib, reporter.clone())),
        };

        let conn = self.brain_connection.get_mut();
        conn.execute_command(command).await?;

        Ok(())
    }

    async fn pairing_pin(&mut self, pin: [u8; 4]) -> v5d_interface::Result {
        let connection = self.get_bluetooth()?;
        connection
            .authenticate_pairing(pin)
            .await
            .map_err(GenericError::from)?;
        Ok(())
    }

    async fn reconnect(&mut self) -> v5d_interface::Result {
        let connection = self.brain_connection.get_mut();
        *connection = setup_connection(self.connection_type).await?;

        Ok(())
    }

    async fn request_pair(&mut self) -> v5d_interface::Result {
        let connection = self.get_bluetooth()?;
        connection
            .request_pairing()
            .await
            .map_err(GenericError::from)?;
        Ok(())
    }

    async fn shutdown(&mut self) -> v5d_interface::Result {
        info!("Received shutdown command");
        self.cancel_token.cancel();
        Ok(())
    }
}
