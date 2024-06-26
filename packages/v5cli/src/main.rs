use tokio::io::AsyncWriteExt;
use v5d_interface::DaemonCommand;

#[tokio::main]
async fn main() {
    let mut sock = v5d_interface::connect_to_socket().await.unwrap();
    sock.writable().await.unwrap();
    let message = DaemonCommand::Test("Hello, world!".to_string());
    let message = serde_json::to_string(&message).unwrap();
    sock.write_all(message.as_bytes()).await.unwrap();
    sock.flush().await.unwrap();
}
