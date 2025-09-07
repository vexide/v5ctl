//! Custom packets for communication with the v5d daemon.

use vex_v5_serial::{
    decode::Decode,
    encode::Encode,
    packets::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
};

use crate::{cmds::V5D_CDC, ecmds::{BLE_PIN, CONNECT_REQUEST, CON_TYPE}};

pub mod connection;
pub mod sharing;