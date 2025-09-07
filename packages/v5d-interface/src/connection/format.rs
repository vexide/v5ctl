use serde::{Deserialize, Serialize};

use crate::{TransferProgress, UploadProgramOpts, error::RemoteError};

pub type CompletionResponse = Result<(), RemoteError>;

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    MockTap { x: u16, y: u16 },
    UploadProgram(UploadProgramOpts),
    Shutdown,
    RequestPair,
    PairingPin([u8; 4]),
    Reconnect,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    BasicAck { successful: bool },
    TransferProgress(TransferProgress),
    TransferComplete(Result<(), String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransferProgressResponse {
    Progress(TransferProgress),
    Complete(CompletionResponse),
}
