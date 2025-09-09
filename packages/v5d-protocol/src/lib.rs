//! Extensions to [`vex-v5-serial`](vex_v5_serial)

pub mod packets;
pub mod connection;
pub mod commands;

/// Custom CDC command identifiers for communication with v5d.
pub(crate) mod cmds {
    /// Command ID for sending a request to the v5d daemon.
    pub const V5D_CDC: u8 = 0xF0;
}
/// Custom CDC2 command identifiers
pub(crate) mod ecmds {
    // Init connection commands
    pub const CONNECT_REQUEST: u8 = 0x01;
    pub const BLE_PIN: u8 = 0x02;

    // Connection info commands
    pub const CON_TYPE: u8 = 0x10;

    // FIFO commands
    pub const FIFO_READ: u8 = 0x20;
    pub const FIFO_WRITE: u8 = 0x21;

    // Connection sharing commands
    pub const CON_LOCK: u8 = 0x30;
}