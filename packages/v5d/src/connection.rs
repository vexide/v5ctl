use std::time::Duration;

use log::{info, warn};
use tokio::{select, time::sleep};
use vex_v5_serial::{
    connection::{
        bluetooth::{self, BluetoothConnection},
        serial::{self, SerialConnection},
        Connection, ConnectionError, ConnectionType,
    },
    decode::Decode,
    encode::Encode,
};

use crate::daemon::DaemonError;

pub enum GenericConnection {
    Bluetooth(BluetoothConnection),
    Serial(SerialConnection),
}
impl Connection for GenericConnection {
    fn connection_type(&self) -> ConnectionType {
        match self {
            GenericConnection::Bluetooth(_) => ConnectionType::Bluetooth,
            GenericConnection::Serial(s) => s.connection_type(),
        }
    }

    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.send_packet(packet).await,
            GenericConnection::Serial(s) => s.send_packet(packet).await,
        }
    }

    async fn receive_packet<P: Decode>(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<P, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.receive_packet(timeout).await,
            GenericConnection::Serial(s) => s.receive_packet(timeout).await,
        }
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.read_user(buf).await,
            GenericConnection::Serial(s) => s.read_user(buf).await,
        }
    }

    async fn write_user(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.write_user(buf).await,
            GenericConnection::Serial(s) => s.write_user(buf).await,
        }
    }
}

async fn bluetooth_connection() -> Result<GenericConnection, DaemonError> {
    // Scan for 10 seconds
    let devices = bluetooth::find_devices(Duration::from_secs(10), None).await?;
    // Open a connection to the first device
    let connection = devices[0].connect().await?;
    info!("Connected to the Brain over Bluetooth!");
    Ok(GenericConnection::Bluetooth(connection))
}

async fn serial_connection() -> Result<GenericConnection, DaemonError> {
    loop {
        // Find all connected serial devices
        let mut devices = serial::find_devices()?.into_iter();
        // Open a connection to the first device
        let Some(connection) = devices.next() else {
            warn!("No serial devices found. Retrying in 100ms...");
            sleep(Duration::from_millis(100)).await;
            continue;
        };
        let connection = SerialConnection::open(connection, Duration::from_secs(2))?;
        info!("Connected to the Brain over serial!");
        return Ok(GenericConnection::Serial(connection));
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
