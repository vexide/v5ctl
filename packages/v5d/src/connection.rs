use std::time::Duration;

use interprocess::local_socket::{GenericNamespaced, Name, ToNsName};
use log::{info, warn};
use tokio::join;
use vex_v5_serial::connection::{
    bluetooth,
    generic::{GenericConnection, GenericError},
    serial,
};

use crate::daemon::DaemonError;

async fn bluetooth_connection() -> Result<GenericConnection, DaemonError> {
    // Scan for 10 seconds
    let devices = bluetooth::find_devices(Duration::from_secs(10), None)
        .await
        .map_err(Into::<GenericError>::into)?;
    // Open a connection to the first device
    let connection = devices[0]
        .connect()
        .await
        .map_err(Into::<GenericError>::into)?;
    info!("Connected to the Brain over Bluetooth!");
    Ok(connection.into())
}

async fn serial_connection() -> Result<Vec<GenericConnection>, DaemonError> {
    // Find all connected serial devices
    Ok(serial::find_devices()
        .map_err(Into::<GenericError>::into)?
        .into_iter()
        .filter_map(|d| d.connect(Duration::from_secs(1)).map(|c| c.into()).ok())
        .collect())
}

pub async fn setup_connections() -> Vec<GenericConnection> {
    // Join the two connection methods
    let (bluetooth, serial) = join! {
        bluetooth_connection(),
        serial_connection(),
    };

    let mut connections = vec![];
    if let Ok(b) = bluetooth {
        connections.push(b);
    } else {
        warn!("No Bluetooth connection to the Brain could be established.");
    }
    if let Ok(s) = serial {
        connections.extend(s);
    } else {
        warn!("No Serial connections to the Brain could be established.");
    }

    connections
}

pub fn get_socket_name() -> Name<'static> {
    "vexide-v5d.sock"
        .to_ns_name::<GenericNamespaced>()
        .expect("socket name should be valid")
}
