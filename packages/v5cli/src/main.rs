use std::{io, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use itertools::Itertools;
use log::{error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use v5d_interface::{AfterFileUpload, DaemonCommand, DaemonResponse, ProgramData};

#[derive(Parser)]
#[command(version, about = "A CLI for interacting with the V5 Daemon (v5d)")]
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
        /// The name of the program
        name: String,
        /// The icon to appear on the program
        icon: ProgramIcon,

        #[arg(short, long)]
        /// The description of the program
        description: Option<String>,
        #[arg(short, long)]
        /// The text to appear in the program type box
        program_type: Option<String>,
        #[arg(short, long)]
        /// Whether or not the program should be compressed before uploading
        uncompressed: bool,
        #[arg(short, long, default_value = "show-screen")]
        /// Action to perform after uploading the program
        after_upload: AfterUpload,

        #[arg(short = 's', long, required_unless_present = "cold")]
        /// Path to the hot bin to upload
        /// If cold is not provided, you must provide this
        hot: Option<PathBuf>,
        #[arg(short = 'c', long, required_unless_present = "hot")]
        /// Path to the cold bin to upload
        /// If hot is not provided, you must provide this
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
async fn get_response(stream: &mut UnixStream) -> io::Result<Vec<DaemonResponse>> {
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    let responses = response.lines().map(serde_json::from_str).try_collect()?;
    Ok(responses)
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
            uncompressed,
            after_upload,
        } => {
            let data = match (hot, cold) {
                (None, None) => unreachable!(),
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
                compression: !uncompressed,
                after_upload: after_upload.into(),
                data,
            };
            write_command(&mut sock, command).await?;

            'outer: loop {
                let responses = get_response(&mut sock).await?;
                for response in responses {
                    match response {
                        DaemonResponse::TransferProgress { percent, step } => {
                            info!("{}: {:.2}%", step, percent)
                        }
                        DaemonResponse::TransferComplete(res) => {
                            if let Err(err) = res {
                                error!("Failed to upload program: {}", err);
                            } else {
                                info!("Successfully uploaded program!");
                            }
                            break 'outer;
                        }
                        _ => panic!("Unexpected response from daemon"),
                    }
                }
            }
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
