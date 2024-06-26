use std::{default, fmt::format, io, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use log::info;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use v5d_interface::{AfterFileUpload, DaemonCommand, DaemonResponse, ProgramData};

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
enum AfterUpload {
    #[default]
    None,
    Run,
    ShowScreen,
}
impl From<AfterUpload> for AfterFileUpload {
    fn from(value: AfterUpload) -> Self {
        match value {
            AfterUpload::None => AfterFileUpload::DoNothing,
            AfterUpload::Run => AfterFileUpload::RunProgram,
            AfterUpload::ShowScreen => AfterFileUpload::ShowRunScreen,
        }
    }
}

#[derive(Debug, ValueEnum, Clone, Copy)]
#[repr(u16)]
enum ProgramIcon {
    VexCodingStudio = 0,
    CoolX = 1,
    /// This is the icon that appears when you provide a missing icon name.
    /// 2 is one such icon that doesn't exist.
    QuestionMark = 2,
    Pizza = 3,
    Clawbot = 10,
    Robot = 11,
    PowerButton = 12,
    Planets = 13,
    Alien = 27,
    AlienInUfo = 29,
    CupInField = 50,
    CupAndBall = 51,
    Matlab = 901,
    Pros = 902,
    RobotMesh = 903,
    RobotMeshCpp = 911,
    RobotMeshBlockly = 912,
    RobotMeshFlowol = 913,
    RobotMeshJS = 914,
    RobotMeshPy = 915,
    /// This icon is duplicated several times and has many file names.
    CodeFile = 920,
    VexcodeBrackets = 921,
    VexcodeBlocks = 922,
    VexcodePython = 925,
    VexcodeCpp = 926,
}

#[derive(Subcommand)]
enum Action {
    MockTap {
        x: u16,
        y: u16,
    },
    UploadProgram {
        /// The slot to upload to
        slot: u8,

        #[arg(short, long)]
        /// The file name of the icon to appear on the program
        icon: ProgramIcon,
        #[arg(short, long)]
        /// The description of the program
        description: Option<String>,
        #[arg(short, long)]
        /// The name of the program
        name: String,
        #[arg(short, long)]
        /// The text to appear in the program type box
        program_type: Option<String>,
        #[arg(long, default_value_t = true)]
        /// Whether or not the program should be compressed before uploading
        compression: bool,
        #[arg(short, long, default_value = "AfterUpload::None")]
        /// Action to perform after uploading the program
        after_upload: AfterUpload,

        #[arg(long)]
        hot: Option<PathBuf>,
        #[arg(long)]
        cold: Option<PathBuf>,
    },
    StopDaemon,
    Reconnect,
}

async fn write_command(stream: &mut UnixStream, cmd: DaemonCommand) -> io::Result<()> {
    let content = serde_json::to_string(&cmd)?;
    stream.write_all(content.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}
async fn get_response(stream: &mut UnixStream) -> io::Result<DaemonResponse> {
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    let response: DaemonResponse = serde_json::from_str(&response)?;
    Ok(response)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _ = simplelog::TermLogger::init(
        log::LevelFilter::Info,
        Default::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    let mut sock = v5d_interface::connect_to_socket()
        .await
        .expect("Failed to connect to v5d! Is it running?");
    match args.action {
        Action::MockTap { x, y } => {
            write_command(&mut sock, DaemonCommand::MockTap { x, y }).await?;
            let response = get_response(&mut sock).await?;
            info!("Received response: {:?}", response);
        }
        Action::UploadProgram {
            slot,
            icon,
            description,
            name,
            program_type,
            hot,
            cold,
            compression,
            after_upload,
        } => {
            let data = match (hot, cold) {
                (None, None) => todo!("I need at least one file to upload!"),
                (None, Some(cold)) => ProgramData::encode_cold(std::fs::read(cold)?),
                (Some(hot), None) => ProgramData::encode_hot(std::fs::read(hot)?),
                (Some(hot), Some(cold)) => {
                    ProgramData::encode_both(std::fs::read(hot)?, std::fs::read(cold)?)
                }
            };
            let description = description.unwrap_or_else(|| "Uploaded with v5d".to_string());
            let program_type = program_type.unwrap_or_else(|| "Unknown".to_string());
            let command = DaemonCommand::UploadProgram {
                name,
                description,
                icon: format!("USER{:03}x.bmp", icon as u16),
                program_type,
                slot,
                compression,
                after_upload: after_upload.into(),
                data,
            };
            write_command(&mut sock, command).await?;
        }
        Action::StopDaemon => {
            write_command(&mut sock, DaemonCommand::Shutdown).await?;
        }
        Action::Reconnect => {
            write_command(&mut sock, DaemonCommand::Reconnect).await?;
        }
    }

    anyhow::Ok(())
}
