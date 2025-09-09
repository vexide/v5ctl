use vex_v5_serial::{
    decode::{Decode, DecodeError},
    encode::Encode,
    packets::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
};

use crate::{
    cmds::V5D_CDC,
    ecmds::{BLE_PIN, CON_TYPE, CONNECT_REQUEST},
};

pub type ConnectionTypePacket = Cdc2CommandPacket<V5D_CDC, CON_TYPE, ()>;
pub type ConnectionTypeReplyPacket = Cdc2ReplyPacket<V5D_CDC, CON_TYPE, ConnectedType>;

pub type ConnectRequestPacket = Cdc2CommandPacket<V5D_CDC, CONNECT_REQUEST, ConnectRequestPayload>;
pub type ConnectRequestReplyPacket = Cdc2ReplyPacket<V5D_CDC, CONNECT_REQUEST, ConnectedType>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ConnectedType {
    Serial = 0,
    Bluetooth = 1,
    NoConnection = 255,
}
impl Decode for ConnectedType {
    fn decode(
        data: impl IntoIterator<Item = u8>,
    ) -> Result<Self, vex_v5_serial::decode::DecodeError> {
        let byte = u8::decode(data)?;
        match byte {
            0 => Ok(ConnectedType::Serial),
            1 => Ok(ConnectedType::Bluetooth),
            255 => Ok(ConnectedType::NoConnection),
            _ => Err(vex_v5_serial::decode::DecodeError::UnexpectedValue {
                value: byte,
                expected: &[0, 1],
            }),
        }
    }
}
impl Encode for ConnectedType {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(vec![*self as u8])
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
impl Decode for ConnectionTypes {
    fn decode(
        data: impl IntoIterator<Item = u8>,
    ) -> Result<Self, vex_v5_serial::decode::DecodeError> {
        let byte = u8::decode(data)?;
        ConnectionTypes::from_bits(byte).ok_or(
            vex_v5_serial::decode::DecodeError::UnexpectedValue {
                value: byte,
                expected: &[0, 1, 2, 3],
            },
        )
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
impl Decode for ConnectRequestPayload {
    fn decode(
        data: impl IntoIterator<Item = u8>,
    ) -> Result<Self, vex_v5_serial::decode::DecodeError> {
        let allowed_types = ConnectionTypes::decode(data)?;
        Ok(ConnectRequestPayload { allowed_types })
    }
}

pub type BluetoothPinPacket = Cdc2CommandPacket<V5D_CDC, BLE_PIN, BluetoothPinPayload>;
pub type BluetoothPinReplyPacket = Cdc2ReplyPacket<V5D_CDC, BLE_PIN, BluetoothPinResult>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BluetoothPinPayload {
    pub pin_bytes: [u8; 4],
}
impl Encode for BluetoothPinPayload {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(self.pin_bytes.to_vec())
    }
}
impl Decode for BluetoothPinPayload {
    fn decode(
        data: impl IntoIterator<Item = u8>,
    ) -> Result<Self, vex_v5_serial::decode::DecodeError> {
        let bytes = data.into_iter().take(4).collect::<Vec<_>>();
        if bytes.len() != 4 {
            return Err(DecodeError::PacketTooShort);
        }
        let mut pin_bytes = [0u8; 4];
        pin_bytes.copy_from_slice(&bytes);
        Ok(BluetoothPinPayload { pin_bytes })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum BluetoothPinResult {
    Success = 0,
    IncorrectPin = 1,
}
impl Encode for BluetoothPinResult {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(vec![*self as u8])
    }
}
impl Decode for BluetoothPinResult {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut iter = data.into_iter();
        let res = u8::decode(&mut iter)?;
        match res {
            0 => Ok(BluetoothPinResult::Success),
            1 => Ok(BluetoothPinResult::IncorrectPin),
            _ => Err(DecodeError::UnexpectedValue {
                value: res,
                expected: &[0, 1],
            }),
        }
    }
}