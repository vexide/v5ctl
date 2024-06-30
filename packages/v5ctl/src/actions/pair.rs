use log::{error, info};
use rustyline::DefaultEditor;
use tokio::{io::BufReader, net::UnixStream};
use v5d_interface::{DaemonCommand, DaemonResponse};

use crate::{get_response, write_command};

pub async fn pair(socket: &mut BufReader<UnixStream>) -> anyhow::Result<()> {
    write_command(socket, DaemonCommand::RequestPair).await?;
    let response = get_response(socket).await?;
    match response {
        DaemonResponse::BasicAck { successful } => {
            if successful {
                info!("Pairing request sent successfully");
            } else {
                error!("Failed to send pairing request");
                return Ok(());
            }
        }
        _ => {
            error!("Unexpected response from daemon");
            return Ok(());
        }
    }

    info!("Enter the pairing pin shown on the brain:");
    let mut editor = DefaultEditor::new().unwrap();
    let pin = editor.readline("Enter PIN: >> ").unwrap();

    let mut chars = pin.chars();

    let mut socket = BufReader::new(v5d_interface::connect_to_socket().await?);

    write_command(
        &mut socket,
        DaemonCommand::PairingPin([
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
        ]),
    )
    .await?;
    let response = get_response(&mut socket).await?;
    match response {
        DaemonResponse::BasicAck { successful } => {
            if successful {
                info!("Pairing successful");
            } else {
                error!("Pairing failed");
            }
        }
        _ => {
            error!("Unexpected response from daemon");
        }
    }

    Ok(())
}
