use std::io::Write;

fn main() {
    let mut sock = v5d_interface::connect_to_socket().unwrap();
    sock.write_all(b"Hello, world!").unwrap();
}
