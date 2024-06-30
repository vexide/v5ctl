use std::{io, path::PathBuf};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;
use vex_v5_serial::packets::file::FileExitAction;

pub use vex_v5_serial::commands::file::ProgramData;

pub fn socket_path() -> PathBuf {
    dirs_next::runtime_dir()
        .expect("Currently, only Linux is supported by the V5 Daemon")
        .join("v5d.sock")
}

pub async fn connect_to_socket() -> io::Result<UnixStream> {
    let path = socket_path();
    debug!("Connecting to UNIX socket at {:?}", path);

    let socket = UnixStream::connect(&path).await?;

    info!("Connected to UNIX socket at {:?}", path);
    Ok(socket)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AfterFileUpload {
    DoNothing,
    RunProgram,
    ShowRunScreen,
    Halt,
}
impl From<AfterFileUpload> for FileExitAction {
    fn from(value: AfterFileUpload) -> Self {
        match value {
            AfterFileUpload::DoNothing => FileExitAction::DoNothing,
            AfterFileUpload::RunProgram => FileExitAction::RunProgram,
            AfterFileUpload::ShowRunScreen => FileExitAction::ShowRunScreen,
            AfterFileUpload::Halt => FileExitAction::Halt,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum UploadStep {
    Ini,
    Monolith,
    Cold,
    Hot,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    MockTap {
        x: u16,
        y: u16,
    },
    UploadProgram {
        name: String,
        description: String,
        icon: String,
        program_type: String,
        // 1-indexed slot
        slot: u8,
        compression: bool,
        after_upload: AfterFileUpload,
        data: ProgramData,
    },
    Shutdown,
    RequestPair,
    PairingPin([u8; 4]),
    Reconnect,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    BasicAck { successful: bool },
    TransferProgress { percent: f32, step: UploadStep },
    TransferComplete(Result<(), String>),
}
