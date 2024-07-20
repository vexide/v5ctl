use std::time::Duration;

use log::{info, warn};
use tokio::{select, time::sleep};
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

async fn serial_connection() -> Result<GenericConnection, DaemonError> {
    loop {
        // Find all connected serial devices
        let mut devices = serial::find_devices()
            .map_err(Into::<GenericError>::into)?
            .into_iter();
        // Open a connection to the first device
        let Some(device) = devices.next() else {
            warn!("No serial devices found. Retrying in 1s...");
            sleep(Duration::from_millis(1000)).await;
            continue;
        };
        let connection = device
            .connect(Duration::from_secs(2))
            .map_err(Into::<GenericError>::into)?;
        info!("Connected to the Brain over serial!");
        return Ok(connection.into());
    }
}

pub async fn setup_connection(
    connection_type: super::ConnectionType,
) -> Result<GenericConnection, DaemonError> {
    match connection_type {
        super::ConnectionType::Bluetooth => bluetooth_connection().await,
        super::ConnectionType::Serial => serial_connection().await,
        super::ConnectionType::Auto => {
            // Race the two connection methods
            select! {
                con = bluetooth_connection() => con,
                con = serial_connection() => con,
            }
        }
    }
}
