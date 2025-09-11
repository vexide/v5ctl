use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
    sync::Arc,
};

use interprocess::local_socket::{ListenerOptions, traits::tokio::Listener};
use log::info;
use snafu::Snafu;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::error;
use vex_v5_serial::{
    commands::screen::MockTap,
    connection::{
        Connection,
        bluetooth::BluetoothConnection,
        generic::{GenericConnection, GenericError},
    },
};

use crate::{
    ConnectionType,
    connection::{get_socket_name, setup_connections},
};

#[derive(Debug, Snafu)]
pub enum DaemonError {
    #[snafu(transparent)]
    Connection { source: GenericError },
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
    cancel_token: CancellationToken,
}
impl Daemon {
    pub async fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            cancel_token: CancellationToken::new(),
        })
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub async fn run(self) {
        let connections: Arc<Mutex<BTreeMap<u32, Mutex<GenericConnection>>>> = Default::default();

        // Brain connection worker
        tokio::spawn({
            let connections = connections.clone();
            async move {
                let mut counter: u32 = 0;
                loop {
                    let new_connections = setup_connections().await;
                    let num_connections = new_connections.len();

                    if num_connections == 0 {
                        error!("Error setting up connection to Brain. Retrying in 1s...");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    };

                    let mut map = connections.lock().await;

                    // add each connection to the connections map
                    new_connections
                        .into_iter()
                        .map(Mutex::new)
                        .enumerate()
                        .for_each(|(i, con)| {
                            map.insert(i as u32 + counter, con);
                        });

                    counter += num_connections as u32;
                }
            }
        });

        let listener = ListenerOptions::new()
            .name(get_socket_name())
            .create_tokio()
            .expect("err creating socket");
        // .map_err(|err| {
        //     if err.kind() == ErrorKind::AddrInUse {
        //         // ExistingServerSnafu.into_error(err)
        //         todo!()
        //     } else {
        //         err.into()
        //     }
        // })?;

        let token = self.cancel_token.clone();

        let handle_new_connection =
            move |res: std::io::Result<interprocess::local_socket::tokio::Stream>| {
                let Ok(stream) = res else {
                    error!("There was an error with an incoming connection");
                    return;
                };

                // Handler for the new connection
                tokio::spawn(async move {
                    info!("New connection established");
                });
            };

        tokio::select! {
            _ = token.cancelled() => {}
            res = listener.accept() => {
                handle_new_connection(res);
            }
        }
    }

    // fn get_bluetooth(&mut self) -> Result<&mut BluetoothConnection, DaemonError> {
    //     let connection = self.brain_connection.get_mut();

    //     if let GenericConnection::Bluetooth(connection) = connection {
    //         Ok(connection)
    //     } else {
    //         WrongConnectionTypeSnafu {
    //             actual: connection.connection_type(),
    //             required: vex_v5_serial::connection::ConnectionType::Bluetooth,
    //         }
    //         .fail()?
    //     }
    // }
}
