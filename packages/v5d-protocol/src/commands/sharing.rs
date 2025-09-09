use std::time::Duration;

use vex_v5_serial::{
    commands::Command,
    packets::cdc2::{Cdc2Ack, Cdc2CommandPacket},
};

use crate::packets::{
    connection::{
        BluetoothPinPacket, BluetoothPinPayload, BluetoothPinReplyPacket, ConnectRequestPacket,
        ConnectRequestPayload, ConnectRequestReplyPacket, ConnectedType, ConnectionTypePacket,
        ConnectionTypeReplyPacket, ConnectionTypes,
    },
    sharing::{ConnectionLockPacket, ConnectionLockReplyPacket, LockAction},
};

pub struct LockConnection {
    pub lock_timeout: Option<u32>,
}
impl Command for LockConnection {
    type Output = ();

    async fn execute<C: vex_v5_serial::connection::Connection + ?Sized>(
        self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        let packet: ConnectionLockPacket = Cdc2CommandPacket::new(LockAction::Lock {
            timeout: self.lock_timeout.unwrap_or(0),
        });

        connection.send_packet(packet).await?;

        let response = connection
            .receive_packet::<ConnectionLockReplyPacket>(
                self.lock_timeout
                    .map(Into::into)
                    .map(Duration::from_millis)
                    .unwrap_or(Duration::MAX),
            )
            .await?;

        match response.try_into_inner()? {
            crate::packets::sharing::LockResult::Success => Ok(()),
            crate::packets::sharing::LockResult::LockTimeout => Err(Cdc2Ack::Timeout.into()),
        }
    }
}

pub struct StartConnection {
    pub lock_timeout: Option<u32>,
    pub prefered_connection_types: ConnectionTypes,
    pub bluetooth_pin: Option<[u8; 4]>,
}
impl Command for StartConnection {
    type Output = ();

    async fn execute<C: vex_v5_serial::connection::Connection + ?Sized>(
        self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        // If we are already connected, do nothing except lock the connection
        let status = connection
            .packet_handshake::<ConnectionTypeReplyPacket>(
                Duration::from_millis(100),
                1,
                ConnectionTypePacket::new(()),
            )
            .await?
            .try_into_inner()?;

        if status == ConnectedType::NoConnection {
            let res = connection
                .packet_handshake::<ConnectRequestReplyPacket>(
                    Duration::from_secs(5),
                    3,
                    ConnectRequestPacket::new(ConnectRequestPayload {
                        allowed_types: self.prefered_connection_types,
                    }),
                )
                .await?
                .try_into_inner()?;

            // If we connected via bluetooth, and a pin was provided, send the pin
            if res == ConnectedType::Bluetooth
                && let Some(pin) = self.bluetooth_pin
            {
                let res = connection
                    .packet_handshake::<BluetoothPinReplyPacket>(
                        Duration::from_millis(100),
                        1,
                        BluetoothPinPacket::new(BluetoothPinPayload { pin_bytes: pin }),
                    )
                    .await?
                    .try_into_inner()?;

                if res != crate::packets::connection::BluetoothPinResult::Success {
                    return Err(Cdc2Ack::Nack.into());
                }
            } else {
                return Err(Cdc2Ack::Nack.into());
            }
        }

        // Now lock the connection
        connection
            .execute_command(LockConnection {
                lock_timeout: self.lock_timeout,
            })
            .await?;

        Ok(())
    }
}

pub struct ReleaseConnection;
impl Command for ReleaseConnection {
    type Output = ();

    async fn execute<C: vex_v5_serial::connection::Connection + ?Sized>(
        self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        let packet: ConnectionLockPacket = Cdc2CommandPacket::new(LockAction::Unlock);

        connection
            .packet_handshake::<ConnectionLockReplyPacket>(Duration::from_millis(100), 1, packet)
            .await?;

        Ok(())
    }
}
