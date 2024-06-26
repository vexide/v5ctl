use clap::{Parser, Subcommand};
use tokio::io::AsyncWriteExt;
use v5d_interface::DaemonCommand;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    MockTap { x: u16, y: u16 },
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
            let content = serde_json::to_string(&DaemonCommand::MockTap { x, y })?;
            sock.write_all(content.as_bytes()).await?;
        }
    }

    anyhow::Ok(())
}
