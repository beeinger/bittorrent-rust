use sha1::{Digest, Sha1};
use std::fmt::Display;
use std::path::PathBuf;
use std::vec;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::{handshake, info, peers};

const BLOCK_SIZE: u32 = 16 * 1_024;

pub async fn download_piece(
    path: PathBuf,
    piece_index: usize,
    output_path: PathBuf,
) -> Result<(), std::io::Error> {
    let metadata = info::get_info(path.clone());
    let peers = peers::get_peers(metadata.clone()).await;
    let piece_hashes = metadata.info.get_piece_hashes();

    assert!(piece_index < piece_hashes.len(), "Piece index out of range");
    let piece_hash = &piece_hashes[piece_index];
    let piece_index = piece_index as u32;

    //? Handshake
    let (_, mut stream) = handshake::get_handshake(metadata.clone(), peers[0].as_str()).await;

    //? Bitfield message
    let message = receive_message(&mut stream).await?;
    assert!(message.id == 5);

    //? Send interested message
    stream
        .write_all(&[0, 0, 0, 1, 2])
        .await
        .expect("Failed to send message");

    //? Unchoke message
    let message = receive_message(&mut stream).await?;
    assert!(message.id == 1);

    let pieces_count =
        (metadata.info.length as f32 / metadata.info.piece_length as f32).ceil() as u32;
    let piece_length = if piece_index == pieces_count - 1 {
        metadata.info.length - (piece_index * metadata.info.piece_length)
    } else {
        metadata.info.piece_length
    };

    //? Piece blocks messages to send
    let piece_blocks_messages = get_piece_blocks_messages(piece_index, piece_length);

    //? Received piece blocks
    let piece_blocks = receive_piece_blocks(&mut stream, piece_blocks_messages).await;

    //? Combine piece blocks into piece=
    let mut piece = vec![0; piece_length as usize];
    for block_enum in piece_blocks.iter().enumerate() {
        let block = block_enum.1.clone().unwrap_or(Block {
            piece_index,
            begin: block_enum.0 as u32 * BLOCK_SIZE,
            block: vec![0; BLOCK_SIZE as usize],
        });
        let start = block.begin as usize;
        let mut block = block.block;
        let mut end = start + block.len();
        if end > piece.len() {
            end = piece.len();
            block = block[0..end - start].to_vec();
        }
        piece[start..end].copy_from_slice(&block);
    }

    //? Hash piece
    let mut hasher = Sha1::new();
    hasher.update(&piece);
    let hash: [u8; 20] = hasher.finalize().into();
    let hash = hex::encode(hash);

    assert_eq!(hash, *piece_hash, "Hashes do not match");

    tokio::fs::write(output_path, piece.clone())
        .await
        .expect("Failed to write piece");

    Ok(())
}

async fn receive_piece_blocks(
    stream: &mut TcpStream,
    piece_blocks_messages: Vec<Vec<u8>>,
) -> Vec<Option<Block>> {
    //? Save the number of chunks
    let number_of_chunks = piece_blocks_messages.len() as u32;
    let piece_blocks_messages = &mut piece_blocks_messages.into_iter();

    //? Send first 5 requests
    for _ in 0..5 {
        if let Some(message) = piece_blocks_messages.next() {
            stream
                .write_all(&message)
                .await
                .expect("Failed to send message");
        }
    }

    let mut blocks = vec![Option::None; number_of_chunks as usize];
    for _ in 0..number_of_chunks {
        let message = match receive_message(stream).await {
            Ok(message) => {
                if message.id != 7 {
                    continue;
                }
                message
            }
            Err(e) => {
                println!("Failed to receive message {}", e);
                continue;
            }
        };

        let block = Block {
            piece_index: bytes_to_u32(&message.payload[0..4]),
            begin: bytes_to_u32(&message.payload[4..8]),
            block: message.payload[8..].to_vec(),
        };

        let block_index = (block.begin as f64 / BLOCK_SIZE as f64).ceil() as u32;
        assert!(block_index < number_of_chunks, "Block index out of range");

        //? Save block
        blocks[block_index as usize] = Some(block.clone());

        //? Send next request to always have 5 requests in flight
        if let Some(message) = piece_blocks_messages.next() {
            stream
                .write_all(&message)
                .await
                .expect("Failed to send message");
        }
    }
    blocks
}

fn get_piece_blocks_messages(piece_index: u32, piece_length: u32) -> Vec<Vec<u8>> {
    let chunks: u32 = (piece_length as f64 / BLOCK_SIZE as f64).ceil() as u32;

    let mut messages_to_send = Vec::new();

    for i in 0..chunks {
        let u32_payload = &[
            piece_index,
            i * BLOCK_SIZE,
            if i == chunks - 1 {
                piece_length - (i * BLOCK_SIZE)
            } else {
                BLOCK_SIZE
            },
        ];
        let payload = u32_slice_to_bytes(u32_payload);
        let mut message = u32_slice_to_bytes(&[payload.len() as u32]);
        message.push(6);
        message.extend(payload);
        messages_to_send.push(message);
    }

    messages_to_send
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

pub async fn receive_message(stream: &mut TcpStream) -> Result<Message, std::io::Error> {
    let message_length: u32 = stream.read_u32().await?;
    let message_id = stream.read_u8().await?;

    let msg = if message_length > 1 {
        let mut msg = vec![0; message_length as usize - 1];
        stream
            .read_exact(&mut msg)
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

    Ok(message)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Block {
    piece_index: u32,
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
