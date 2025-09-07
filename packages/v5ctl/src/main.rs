use std::path::PathBuf;

use actions::upload::{AfterUpload, ProgramIcon};
use clap::{Parser, Subcommand};
use log::info;
use tokio::io::BufReader;
use v5d_interface::{connection::DaemonConnection, DeviceInterface};

pub mod actions;

#[derive(Parser)]
#[command(version, about = "A CLI for interacting with the V5 Daemon (v5d)")]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    MockTap {
        x: u16,
        y: u16,
    },
    /// Uploads a user program to the brain
    #[command(name = "upload", visible_alias = "u")]
    UploadProgram {
        /// Path to the monolith bin to upload
        #[arg(required_unless_present_any = ["hot", "cold"], conflicts_with_all = ["hot", "cold"])]
        monolith: Option<PathBuf>,

        /// Path to the hot bin to upload
        #[arg(long, required_unless_present_any = ["cold", "monolith"], conflicts_with = "monolith")]
        hot: Option<PathBuf>,

        /// Path to the cold bin to upload
        #[arg(long, required_unless_present_any = ["hot", "monolith"], conflicts_with = "monolith")]
        cold: Option<PathBuf>,

        /// The slot to upload to
        #[arg(long, short)]
        slot: u8,

        /// The name of the program
        #[arg(short, long)]
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
    Pair,
    StopDaemon,
    Reconnect,
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

    let mut conn = DaemonConnection::new()
        .await
        .expect("Failed to connect to v5d! Is it running?");

    match args.action {
        Action::MockTap { x, y } => {
            conn.mock_tap(x, y).await?;
        }
        Action::UploadProgram {
            slot,
            icon,
            description,
            name,
            program_type,
            monolith,
            hot,
            cold,
            uncompressed,
            after_upload,
        } => {
            actions::upload(
                &mut conn,
                monolith,
                hot,
                cold,
                slot,
                name,
                description,
                icon,
                program_type,
                uncompressed,
                after_upload,
            )
            .await?;
        }
        Action::StopDaemon => {
            conn.shutdown().await?;
        }
        Action::Reconnect => {
            conn.reconnect().await?;
        }
        Action::Pair => {
            actions::pair(&mut conn).await?;
        }
    }

    anyhow::Ok(())
}
