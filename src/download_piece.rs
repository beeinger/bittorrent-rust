use sha1::{Digest, Sha1};
use std::fmt::Display;
use std::path::Path;
use std::vec;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use crate::{handshake, info, peers};

pub const BLOCK_SIZE: u32 = 16 * 1_024;

pub async fn download_piece(
    path: &Path,
    piece_index: usize,
    output_path: &Path,
) -> Result<(), std::io::Error> {
    let metadata = RwLock::new(info::get_info(path));
    let peers = peers::get_peers(&metadata).await;
    let piece_hashes = metadata.write().await.info.get_piece_hashes();

    assert!(piece_index < piece_hashes.len(), "Piece index out of range");
    let piece_hash = &piece_hashes[piece_index];

    //? Handshake
    let (_, stream) = handshake::get_handshake(&metadata, &peers[0]).await;

    let bitmap = get_bitfield(&stream).await;
    assert!(bitmap[piece_index], "Peer does not have piece");

    //? Send interested message
    stream
        .write()
        .await
        .write_all(&[0, 0, 0, 1, 2])
        .await
        .expect("Failed to send message");

    //? Unchoke message
    let message = receive_message(&stream).await?;
    assert!(message.id == 1);

    let metadata_info = &metadata.read().await.info;
    let piece_index = piece_index as u32;
    let pieces_count = piece_hashes.len() as u32;
    let piece_length = if piece_index == pieces_count - 1 {
        metadata_info.length - (piece_index * metadata_info.piece_length)
    } else {
        metadata_info.piece_length
    };

    //? Piece blocks messages to send
    let piece_blocks_messages = get_piece_blocks_messages(piece_index, piece_length);

    //? Received piece blocks
    let piece_blocks = receive_piece_blocks(&stream, piece_blocks_messages).await;

    let piece = combine_blocks_into_piece(piece_blocks, piece_length, piece_index, piece_hash);

    tokio::fs::write(output_path, piece.clone())
        .await
        .expect("Failed to write piece");

    Ok(())
}

pub async fn get_bitfield(stream: &RwLock<TcpStream>) -> Vec<bool> {
    //? Bitfield message
    let message = receive_message(stream)
        .await
        .expect("Failed to receive bitfield message");
    assert!(message.id == 5);

    message
        .payload
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1 == 1))
        .collect()
}

pub fn combine_blocks_into_piece(
    piece_blocks: Vec<Option<Block>>,
    piece_length: u32,
    piece_index: u32,
    piece_hash: &str,
) -> Vec<u8> {
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

    piece
}

pub async fn receive_piece_blocks(
    stream: &RwLock<TcpStream>,
    piece_blocks_messages: Vec<Vec<u8>>,
) -> Vec<Option<Block>> {
    //? Save the number of chunks
    let number_of_chunks = piece_blocks_messages.len() as u32;
    let piece_blocks_messages = &mut piece_blocks_messages.into_iter();

    //? Send first 5 requests
    for _ in 0..5 {
        if let Some(message) = piece_blocks_messages.next() {
            stream
                .write()
                .await
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
                .write()
                .await
                .write_all(&message)
                .await
                .expect("Failed to send message");
        }
    }
    blocks
}

pub fn get_piece_blocks_messages(piece_index: u32, piece_length: u32) -> Vec<Vec<u8>> {
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

pub fn u32_slice_to_bytes(input: &[u32]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 4);
    for &num in input {
        output.push((num >> 24) as u8);
        output.push((num >> 16) as u8);
        output.push((num >> 8) as u8);
        output.push(num as u8);
    }
    output
}

pub fn bytes_to_u32(input: &[u8]) -> u32 {
    ((input[0] as u32) << 24)
        | ((input[1] as u32) << 16)
        | ((input[2] as u32) << 8)
        | (input[3] as u32)
}

pub async fn receive_message(stream: &RwLock<TcpStream>) -> Result<Message, std::io::Error> {
    let message_length: u32 = stream.write().await.read_u32().await?;
    let message_id = stream.write().await.read_u8().await?;

    let msg = if message_length > 1 {
        let mut msg = vec![0; message_length as usize - 1];
        stream
            .write()
            .await
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
    pub piece_index: u32,
    pub begin: u32,
    pub block: Vec<u8>,
}

#[derive(Debug)]
pub struct Message {
    pub length: u32,
    pub id: u8,
    pub payload: Vec<u8>,
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
