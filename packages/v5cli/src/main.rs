use std::io;

use clap::{Parser, Subcommand};
use log::info;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use v5d_interface::{DaemonCommand, DaemonResponse};

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    MockTap { x: u16, y: u16 },
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
        Action::StopDaemon => {
            write_command(&mut sock, DaemonCommand::Shutdown).await?;
        }
        Action::Reconnect => {
            write_command(&mut sock, DaemonCommand::Reconnect).await?;
        }
    }

    anyhow::Ok(())
}
