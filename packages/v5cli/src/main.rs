use std::io::Write;

use v5d_interface::Command;

fn main() {
    let mut sock = v5d_interface::connect_to_socket().unwrap();
    let message  = Command::Test("Hello, world!".to_string());
    let message = serde_json::to_string(&message).unwrap();
    sock.write_all(message.as_bytes()).unwrap();
}
