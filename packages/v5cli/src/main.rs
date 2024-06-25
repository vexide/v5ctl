use std::io::{Read, Write};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use v5d_interface::{DaemonCommand, DaemonResponse};

#[tokio::main]
async fn main() {
    let mut sock = v5d_interface::connect_to_socket().await.unwrap();
    sock.writable().await.unwrap();
    let message  = DaemonCommand::Test("Hello, world!".to_string());
    let message = serde_json::to_string(&message).unwrap();
    sock.write_all(message.as_bytes()).await.unwrap();
    sock.flush().await.unwrap();

    sock.readable().await.unwrap();
    let mut response = String::new();
    sock.read_to_string(&mut response).await.unwrap();
    let response: DaemonResponse = serde_json::from_str(&response).unwrap();
    println!("{:?}", response);
}
