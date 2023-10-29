use sha1::{Digest, Sha1};
use std::fmt::Display;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::{handshake, info, peers};

pub async fn download_piece(path: PathBuf, piece_index: usize, _output_path: PathBuf) -> () {
    let metadata = info::get_info(path);
    let peers = peers::get_peers(metadata.clone()).await;
    let piece_hash = metadata.info.get_piece_hashes()[piece_index].clone();

    let (_, stream) = handshake::get_handshake(metadata.clone(), peers[0].as_str()).await;
    let (message, mut stream) = receive_message(stream).await;
    //? Bitfield message
    assert!(message.id == 5);

    //? Send interested message
    stream
        .write(&[0, 0, 0, 1, 2])
        .await
        .expect("Failed to send message");

    let (message, mut stream) = receive_message(stream).await;
    //? Unchoke message
    assert!(message.id == 1);

    let piece_length = metadata.info.piece_length;
    let chunk_size = 16 * 1024;
    let chunks = piece_length / chunk_size;

    let mut messages_to_send = Vec::new();

    for i in 0..chunks {
        let payload = u32_slice_to_bytes(&[i, i * chunk_size, {
            if i == chunks - 1 {
                piece_length - i * chunk_size
            } else {
                chunk_size
            }
        }]);
        let mut message = u32_slice_to_bytes(&[payload.len() as u32]);
        message.push(6);
        message.extend(payload);
        messages_to_send.push(message);
    }

    let mut messages_to_send = messages_to_send.iter();

    for _ in [0..5] {
        if let Some(message) = messages_to_send.next() {
            stream
                .write(&message)
                .await
                .expect("Failed to send message");
        }
    }

    let mut blocks = Vec::new();
    while blocks.len() != chunks as usize {
        let (message, passed_stream) = receive_message(stream).await;
        stream = passed_stream;
        println!("{}/{} id {}", blocks.len(), chunks, message.id);
        if message.id != 7 {
            continue;
        }

        let block = Block {
            index: bytes_to_u32(&message.payload[0..4]),
            begin: bytes_to_u32(&message.payload[4..8]),
            block: message.payload[8..].to_vec(),
        };

        blocks.push(block);

        if let Some(message) = messages_to_send.next() {
            stream
                .write(&message)
                .await
                .expect("Failed to send message");
        }
    }

    let piece = &blocks
        .iter()
        .fold(Vec::new(), |mut acc, block| {
            acc.extend(block.block.clone());
            acc
        })
        .to_vec()[..piece_length as usize];

    let mut hasher = Sha1::new();
    hasher.update(piece);
    let hash: [u8; 20] = hasher.finalize().into();
    let hash = hex::encode(hash);

    println!("Correct hash: {}", piece_hash);
    println!("Actual hash: {}", hash);

    assert!(hash == piece_hash, "Hashes do not match");
}

fn u32_slice_to_bytes(input: &[u32]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 4);
    for &num in input {
        output.push((num >> 24) as u8);
        output.push((num >> 16) as u8);
        output.push((num >> 8) as u8);
        output.push(num as u8);
    }
    output
}

fn bytes_to_u32(input: &[u8]) -> u32 {
    ((input[0] as u32) << 24)
        | ((input[1] as u32) << 16)
        | ((input[2] as u32) << 8)
        | (input[3] as u32)
}

pub async fn receive_message(mut stream: TcpStream) -> (Message, TcpStream) {
    // let mut msg: Vec<u8> = Vec::new();
    let mut msg = [0; 5];
    stream
        .read(&mut msg)
        .await
        .expect("Failed to read from server");

    let message_length: u32 = bytes_to_u32(&msg[0..4]);
    let message_id = msg[4];

    let msg = if message_length > 0 {
        let mut msg = vec![0; message_length as usize - 1];
        stream
            .read(&mut msg)
            .await
            .expect("Failed to read from server");
        msg
    } else {
        Vec::new()
    };

    let message = Message {
        length: message_length,
        id: message_id,
        payload: msg,
    };

    (message, stream)
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Block {
    index: u32,
    begin: u32,
    block: Vec<u8>,
}

#[derive(Debug)]
pub struct Message {
    length: u32,
    id: u8,
    payload: Vec<u8>,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ length: {}, id: {}, payload: {:?} }}",
            self.length, self.id, self.payload
        )
    }
}
