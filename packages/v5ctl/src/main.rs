use std::{path::PathBuf, time::Duration};

use actions::upload::{AfterUpload, ProgramIcon};
use clap::{Parser, Subcommand};
use v5d_protocol::{
    commands::sharing::StartConnection, connection::DaemonConnection,
    packets::connection::ConnectionTypes,
};
use vex_v5_serial::{
    commands::screen::{MockTap, MockTouch},
    connection::Connection,
};

pub mod actions;

fn validate_pin(s: &str) -> Result<[u8; 4], String> {
    if s.len() != 4 {
        return Err("Must be exactly 4 characters".to_string());
    }

    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err("Must contain only numeric digits".to_string());
    }

    let mut pin_bytes = [0u8; 4];
    let parsed = s.chars().map(|c| c as u8 - b'0').collect::<Vec<_>>();
    pin_bytes.copy_from_slice(&parsed);
    Ok(pin_bytes)
}

#[derive(Parser)]
#[command(version, about = "A CLI for interacting with the V5 Daemon (v5d)")]
struct Args {
    #[clap(subcommand)]
    action: Action,
    #[arg(long, value_parser = validate_pin)]
    bluetooth_pin: Option<[u8; 4]>,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt().init();

    let mut conn = DaemonConnection::new()
        .await
        .expect("Failed to connect to v5d! Is it running?");

    conn.execute_command(StartConnection {
        lock_timeout: Some(500),
        prefered_connection_types: ConnectionTypes::all(),
        bluetooth_pin: args.bluetooth_pin,
    });

    match args.action {
        Action::MockTap { x, y } => conn.execute_command(MockTap { x, y }).await?,
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
    }

    anyhow::Ok(())
}
