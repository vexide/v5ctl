use std::{io, path::PathBuf, time::Instant};

use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{error, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};
use v5d_interface::{AfterFileUpload, DaemonCommand, DaemonResponse, ProgramData, UploadStep};

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

#[derive(Default, Debug, ValueEnum, Clone, Copy)]
#[repr(u16)]
enum ProgramIcon {
    VexCodingStudio = 0,
    CoolX = 1,
    /// This is the icon that appears when you provide a missing icon name.
    /// 2 is one such icon that doesn't exist.
    #[default]
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
    /// Uploads a user program to the brain
    #[command(name = "upload")]
    UploadProgram {
        /// Path to the hot bin to upload
        ///
        /// If cold is not provided, you must provide this
        #[arg(required_unless_present = "cold")]
        hot: Option<PathBuf>,

        /// Path to the cold bin to upload
        ///
        /// If hot is not provided, you must provide this
        #[arg(required_unless_present = "hot")]
        cold: Option<PathBuf>,

        /// The slot to upload to
        #[arg(long)]
        slot: u8,

        /// The name of the program
        #[arg(long, long)]
        name: Option<String>,

        /// The description of the program
        #[arg(short, long)]
        description: Option<String>,

        /// The icon to appear on the program
        #[arg(short, long, default_value = "question-mark")]
        icon: ProgramIcon,

        /// The text to appear in the program type box
        #[arg(short = 't', long)]
        program_type: Option<String>,

        /// Whether or not the program should be compressed before uploading
        #[arg(short, long)]
        uncompressed: bool,

        /// Action to perform after uploading the program
        #[arg(short, long, default_value = "show-screen")]
        after_upload: AfterUpload,
    },
    StopDaemon,
    Reconnect,
}

async fn write_command(stream: &mut BufReader<UnixStream>, cmd: DaemonCommand) -> io::Result<()> {
    let mut content = serde_json::to_string(&cmd)?;
    content.push('\n');
    stream.write_all(content.as_bytes()).await?;
    Ok(())
}
async fn get_response(stream: &mut BufReader<UnixStream>) -> io::Result<DaemonResponse> {
    let mut response = String::new();
    stream.read_line(&mut response).await?;
    let responses = serde_json::from_str(&response)?;
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

    let mut sock = BufReader::new(
        v5d_interface::connect_to_socket()
            .await
            .expect("Failed to connect to v5d! Is it running?"),
    );
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
            let multi_progress = MultiProgress::new();

            let ini_progress = multi_progress.add(ProgressBar::new(10000));
            ini_progress.set_style(
                ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.cyan} {prefix}")
                .unwrap()
                    .progress_chars("█▇▆▅▄▃▂▁ "),
            );
            ini_progress.set_message("INI");

            let cold_progress = if cold.is_some() {
                let bar = multi_progress.add(ProgressBar::new(10000));
                bar.set_style(
                    ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.blue} {prefix}")
                    .unwrap()
                    .progress_chars("█▇▆▅▄▃▂▁ "),
                );
                bar.set_message("COLD");

                Some(bar)
            } else {
                None
            };
            
            let hot_progress = if hot.is_some() {
                let bar = multi_progress.add(ProgressBar::new(10000));
                bar.set_style(
                    ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.red} {prefix}")
                        .unwrap()
                        .progress_chars("█▇▆▅▄▃▂▁ "),
                );

                bar.set_message(if cold.is_none() {
                    "BIN"
                } else {
                    "HOT"
                });

                Some(bar)
            } else {
                None
            };

            let (fallback_name, data) = match (hot, cold) {
                (None, None) => unreachable!(),
                (None, Some(cold)) => (
                    cold.file_stem().unwrap().to_string_lossy().to_string(),
                    ProgramData::encode_cold(std::fs::read(cold)?),
                ),
                (Some(hot), None) => (
                    hot.file_stem().unwrap().to_string_lossy().to_string(),
                    ProgramData::encode_hot(std::fs::read(hot)?),
                ),
                (Some(hot), Some(cold)) => (
                    hot.file_stem().unwrap().to_string_lossy().to_string(),
                    ProgramData::encode_both(std::fs::read(hot)?, std::fs::read(cold)?),
                ),
            };
            let description = description.unwrap_or_else(|| "Uploaded with v5d".to_string());
            let program_type = program_type.unwrap_or_else(|| "Unknown".to_string());
            let command = DaemonCommand::UploadProgram {
                name: name.unwrap_or(fallback_name),
                description,
                icon: format!("USER{:03}x.bmp", icon as u16),
                program_type,
                slot,
                compression: !uncompressed,
                after_upload: after_upload.into(),
                data,
            };
            write_command(&mut sock, command).await?;

            let mut prev_percent: f32 = 0.0;
            let mut prev_step = UploadStep::Ini;
            let mut start = Instant::now();

            'outer: loop {
                let responses = get_response(&mut sock).await?;
                
                for response in responses {
                    match response {
                        DaemonResponse::TransferProgress { percent, step } => {
                            let delta = ((percent - prev_percent) * 100.0) as u64;
                            
                            if prev_step != step {
                                start = Instant::now();
                            }

                            let elapsed = start.elapsed();
                            let elapsed_format = format!("{:.2?}", elapsed);

                            match step {
                                UploadStep::Ini => {
                                    ini_progress.inc(delta);
                                    ini_progress.set_prefix(elapsed_format);
                                },
                                UploadStep::Cold => if let Some(ref cold_progress) = cold_progress {
                                    cold_progress.inc(delta);
                                    cold_progress.set_prefix(elapsed_format);
                                },
                                UploadStep::Hot => if let Some(ref hot_progress) = hot_progress {
                                    hot_progress.inc(delta);
                                    hot_progress.set_prefix(elapsed_format);
                                },
                            }

                            prev_step = step;
                            prev_percent = percent;
                        },
                        DaemonResponse::TransferComplete(res) => {
                            ini_progress.finish();
                            if let Some(ref cold_progress) = cold_progress {
                                cold_progress.finish();
                            }
                            if let Some(ref hot_progress) = hot_progress {
                                hot_progress.finish();
                            }
                            if let Err(err) = res {
                                error!("Failed to upload program: {}", err);
                            } else {
                                info!("Successfully uploaded program!");
                            }
                            break 'outer;
                        }
                        _ => panic!("Unexpected response from daemon"),
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
        Action::StopDaemon => {
            write_command(&mut sock, DaemonCommand::Shutdown).await?;
        }
        Action::Reconnect => {
            write_command(&mut sock, DaemonCommand::Reconnect).await?;
        }
    }

    anyhow::Ok(())
}
