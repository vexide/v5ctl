use vex_v5_serial::{
    decode::{Decode, DecodeError},
    encode::Encode,
    packets::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
};

use crate::{cmds::V5D_CDC, ecmds::CON_LOCK};

pub type ConnectionLockPacket = Cdc2CommandPacket<V5D_CDC, CON_LOCK, LockAction>;
pub type ConnectionLockReplyPacket = Cdc2ReplyPacket<V5D_CDC, CON_LOCK, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LockAction {
    Lock {
        /// Timeout in milliseconds
        /// 0 for no timeout
        timeout: u32,
    },
    Unlock,
}
impl Encode for LockAction {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        match self {
            LockAction::Lock { timeout } => {
                let mut v = vec![0u8];
                v.extend(timeout.to_le_bytes());
                Ok(v)
            }
            LockAction::Unlock => Ok(vec![1u8]),
        }
    }
}
impl Decode for LockAction {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut iter = data.into_iter();
        let action = u8::decode(&mut iter)?;
        match action {
            0 => {
                let timeout = u32::decode(&mut iter)?;
                Ok(LockAction::Lock { timeout })
            }
            1 => Ok(LockAction::Unlock),
            _ => Err(DecodeError::UnexpectedValue {
                value: action,
                expected: &[0, 1],
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum LockResult {
    Success = 0,
    LockTimeout = 1,
}
impl Decode for LockResult {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let byte = u8::decode(data)?;
        match byte {
            0 => Ok(LockResult::Success),
            1 => Ok(LockResult::LockTimeout),
            _ => Err(DecodeError::UnexpectedValue {
                value: byte,
                expected: &[0, 1],
            }),
        }
    }
}
impl Encode for LockResult {
    fn encode(&self) -> Result<Vec<u8>, vex_v5_serial::encode::EncodeError> {
        Ok(vec![*self as u8])
    }
}
