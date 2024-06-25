use std::io::{Read, Write};

use v5d_interface::{DaemonCommand, DaemonResponse};

fn main() {
    let mut sock = v5d_interface::connect_to_socket().unwrap();
    let message  = DaemonCommand::Test("Hello, world!".to_string());
    let message = serde_json::to_string(&message).unwrap();
    sock.write_all(message.as_bytes()).unwrap();

    // let mut response = String::new();
    // // sock.read_to_string(&mut response).unwrap();
    // let response: DaemonResponse = serde_json::from_str(&response).unwrap();
    // println!("{:?}", response);
}
