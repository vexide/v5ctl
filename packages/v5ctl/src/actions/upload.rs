use std::{path::PathBuf, time::Instant};

use clap::ValueEnum;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{error, info};
use tokio::{io::BufReader, net::UnixStream};
use v5d_interface::{
    get_response, send_command, AfterFileUpload, DaemonCommand, DaemonResponse, ProgramData,
    UploadStep,
};

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
pub enum AfterUpload {
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
pub enum ProgramIcon {
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

const PROGRESS_CHARS: &str = "⣿⣦⣀";

pub async fn upload(
    socket: &mut BufReader<UnixStream>,
    monolith: Option<PathBuf>,
    hot: Option<PathBuf>,
    cold: Option<PathBuf>,
    slot: u8,
    name: Option<String>,
    description: Option<String>,
    icon: ProgramIcon,
    program_type: Option<String>,
    uncompressed: bool,
    after_upload: AfterUpload,
) -> anyhow::Result<()> {
    let multi_progress = MultiProgress::new();

    let ini_progress = multi_progress
        .add(ProgressBar::new(10000))
        .with_style(
            ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.green} {prefix}")
                .unwrap()
                .progress_chars(PROGRESS_CHARS),
        )
        .with_message("INI");

    let cold_progress = if cold.is_some() {
        let bar = multi_progress
            .add(ProgressBar::new(10000))
            .with_style(
                ProgressStyle::with_template(
                    "{msg:4} {percent_precise:>7}% {bar:40.blue} {prefix}",
                )
                .unwrap()
                .progress_chars(PROGRESS_CHARS),
            )
            .with_message("COLD");

        Some(bar)
    } else {
        None
    };

    let hot_progress = if hot.is_some() {
        let bar = multi_progress
            .add(ProgressBar::new(10000))
            .with_style(
                ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.red} {prefix}")
                    .unwrap()
                    .progress_chars(PROGRESS_CHARS),
            )
            .with_message("HOT");

        Some(bar)
    } else {
        None
    };

    let monolith_progress = if monolith.is_some() {
        let bar = multi_progress
            .add(ProgressBar::new(10000))
            .with_style(
                ProgressStyle::with_template("{msg:4} {percent_precise:>7}% {bar:40.red} {prefix}")
                    .unwrap()
                    .progress_chars(PROGRESS_CHARS),
            )
            .with_message("BIN");

        Some(bar)
    } else {
        None
    };

    let (fallback_name, data) = match (monolith, cold, hot) {
        (Some(monolith), None, None) => (
            monolith.file_stem().unwrap().to_string_lossy().to_string(),
            ProgramData::Monolith(std::fs::read(monolith)?),
        ),
        (None, None, Some(cold)) => (
            cold.file_stem().unwrap().to_string_lossy().to_string(),
            ProgramData::HotCold {
                hot: None,
                cold: Some(std::fs::read(cold)?),
            },
        ),
        (None, Some(hot), None) => (
            hot.file_stem().unwrap().to_string_lossy().to_string(),
            ProgramData::HotCold {
                hot: Some(std::fs::read(hot)?),
                cold: None,
            },
        ),
        (None, Some(hot), Some(cold)) => (
            hot.file_stem().unwrap().to_string_lossy().to_string(),
            ProgramData::HotCold {
                hot: Some(std::fs::read(hot)?),
                cold: Some(std::fs::read(cold)?),
            },
        ),
        _ => unreachable!(),
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
    send_command(socket, command).await?;

    let mut prev_step = UploadStep::Ini;
    let mut start = Instant::now();

    ini_progress.tick();
    if let Some(ref monolith_progress) = monolith_progress {
        monolith_progress.tick();
    }
    if let Some(ref cold_progress) = cold_progress {
        cold_progress.tick();
    }
    if let Some(ref hot_progress) = hot_progress {
        hot_progress.tick();
    }

    loop {
        let response = get_response(socket).await?;

        match response {
            DaemonResponse::TransferProgress { percent, step } => {
                if prev_step != step {
                    start = Instant::now();
                }

                let elapsed = start.elapsed();
                let elapsed_format = format!("{:.2?}", elapsed);
                let position = (percent * 100.0) as u64;

                match step {
                    UploadStep::Ini => {
                        ini_progress.set_position(position);
                        ini_progress.set_prefix(elapsed_format);
                    }
                    UploadStep::Monolith => {
                        if let Some(ref monolith_progress) = monolith_progress {
                            monolith_progress.set_position(position);
                            monolith_progress.set_prefix(elapsed_format);
                        }
                    }
                    UploadStep::Cold => {
                        if let Some(ref cold_progress) = cold_progress {
                            cold_progress.set_position(position);
                            cold_progress.set_prefix(elapsed_format);
                        }
                    }
                    UploadStep::Hot => {
                        if let Some(ref hot_progress) = hot_progress {
                            hot_progress.set_position(position);
                            hot_progress.set_prefix(elapsed_format);
                        }
                    }
                }

                prev_step = step;
            }
            DaemonResponse::TransferComplete(res) => {
                ini_progress.finish();
                if let Some(ref monolith_progress) = monolith_progress {
                    monolith_progress.finish();
                }
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
                break;
            }
            _ => panic!("Unexpected response from daemon"),
        }
    }

    Ok(())
}
