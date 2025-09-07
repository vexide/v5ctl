use log::{error, info};
use rustyline::DefaultEditor;
use tokio::{io::BufReader, net::UnixStream};
use v5d_interface::{connection::DaemonConnection, DeviceInterface};

pub async fn pair(connection: &mut DaemonConnection) -> anyhow::Result<()> {
    let res = connection.request_pair().await;
    if let Err(err) = res {
        error!("Failed to send pairing request");
        return Err(err);
    } else {
        info!("Pairing request sent successfully");
    }

    info!("Enter the pairing pin shown on the brain:");
    let mut editor = DefaultEditor::new().unwrap();
    let pin = editor.readline("Enter PIN: >> ").unwrap();

    let mut chars = pin.chars();

    connection
        .pairing_pin([
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
            chars.next().unwrap().to_digit(10).unwrap() as u8,
        ])
        .await?;

    Ok(())
}
