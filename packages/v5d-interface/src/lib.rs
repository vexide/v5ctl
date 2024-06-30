use std::{io, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;
use vex_v5_serial::packets::file::FileExitAction;

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

#[derive(Debug, Serialize, Deserialize)]
pub enum ProgramData {
    Hot(String),
    Cold(String),
    Both { hot: String, cold: String },
}
impl ProgramData {
    pub fn decode_both(&self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        match self {
            ProgramData::Hot(hot) => (Some(STANDARD.decode(hot.as_bytes()).unwrap()), None),
            ProgramData::Cold(cold) => (None, Some(STANDARD.decode(cold.as_bytes()).unwrap())),
            ProgramData::Both { hot, cold } => (
                Some(STANDARD.decode(hot.as_bytes()).unwrap()),
                Some(STANDARD.decode(cold.as_bytes()).unwrap()),
            ),
        }
    }
    pub fn encode_hot(hot: Vec<u8>) -> Self {
        ProgramData::Hot(STANDARD.encode(hot))
    }
    pub fn encode_cold(cold: Vec<u8>) -> Self {
        ProgramData::Cold(STANDARD.encode(cold))
    }
    pub fn encode_both(hot: Vec<u8>, cold: Vec<u8>) -> Self {
        ProgramData::Both {
            hot: STANDARD.encode(hot),
            cold: STANDARD.encode(cold),
        }
    }
}
impl From<ProgramData> for vex_v5_serial::commands::file::ProgramData {
    fn from(value: ProgramData) -> Self {
        match value.decode_both() {
            (Some(hot), None) => vex_v5_serial::commands::file::ProgramData::Hot(hot),
            (None, Some(cold)) => vex_v5_serial::commands::file::ProgramData::Cold(cold),
            (Some(hot), Some(cold)) => {
                vex_v5_serial::commands::file::ProgramData::Both { hot, cold }
            }
            (None, None) => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum UploadStep {
    Ini,
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
