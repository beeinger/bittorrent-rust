use crate::info::{self, Metadata};

use std::io::{Read, Write};
use std::net::TcpStream;

pub async fn get_handshake(path: &str, peer: &str) -> String {
    let metadata = info::get_info(path);
    let handshake = construct_handshake(metadata);

    let mut stream = TcpStream::connect(peer).expect("Could not connect to server");
    stream.write(&handshake).expect("Failed to send message");

    let mut buffer = [0; 68];
    stream
        .read(&mut buffer)
        .expect("Failed to read from server");

    buffer[48..]
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}

fn construct_handshake(metadata: Metadata) -> Vec<u8> {
    let mut handshake = Vec::new();
    handshake.push(19);
    handshake.extend(b"BitTorrent protocol");
    handshake.extend(vec![0; 8]);
    handshake.extend(metadata.info.get_hash());
    handshake.extend(b"21372137696921372137");
    handshake
}
