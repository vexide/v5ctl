//! Custom packets for communication with the v5d daemon.

use vex_v5_serial::{
    decode::Decode,
    encode::Encode,
    packets::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
};

use crate::{cmds::V5D_CDC, ecmds::{BLE_PIN, CONNECT_REQUEST, CON_TYPE}};

pub type ConnectRequestPacket = Cdc2CommandPacket<V5D_CDC, CONNECT_REQUEST, ConnectRequestPayload>;
pub type ConnectionRequestReplyPacket = Cdc2ReplyPacket<V5D_CDC, CONNECT_REQUEST, ConnectedType>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ConnectedType {
    Serial = 0,
    Bluetooth = 1,
}
impl Decode for ConnectedType {
    fn decode(
        data: impl IntoIterator<Item = u8>,
    ) -> Result<Self, vex_v5_serial::decode::DecodeError> {
        let byte = u8::decode(data)?;
        match byte {
            0 => Ok(ConnectedType::Serial),
            1 => Ok(ConnectedType::Bluetooth),
            _ => Err(vex_v5_serial::decode::DecodeError::UnexpectedValue {
                value: byte,
                expected: &[0, 1],
            }),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct ConnectionTypes: u8 {
        const SERIAL = 0b01;
        const BLUETOOTH = 0b10;
    }
}
impl Encode for ConnectionTypes {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(vec![self.bits()])
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct ConnectRequestPayload {
    pub allowed_types: ConnectionTypes,
}
impl Encode for ConnectRequestPayload {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        self.allowed_types.encode()
    }
}

pub type BluetoothPinPacket = Cdc2CommandPacket<V5D_CDC, BLE_PIN, BluetoothPinPayload>;
pub type BluetoothPinReplyPacket = Cdc2ReplyPacket<V5D_CDC, BLE_PIN, bool>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BluetoothPinPayload {
    pub pin_bytes: [u8; 4],
}
impl Encode for BluetoothPinPayload {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(self.pin_bytes.to_vec())
    }
}

pub type ConnectionTypePacket = Cdc2CommandPacket<V5D_CDC, CON_TYPE, ()>;
pub type ConnectionTypeReplyPacket = Cdc2ReplyPacket<V5D_CDC, CON_TYPE, ConnectedType>;