use std::path::Path;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{self, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::{download_piece, handshake, info, peers};

#[allow(unused_variables)]
pub async fn download(path: &Path, output_path: &Path) -> Result<(), std::io::Error> {
    let metadata = RwLock::new(info::get_info(path));
    let peers = peers::get_peers(&metadata).await;
    let piece_hashes = metadata.read().await.info.get_piece_hashes();

    let metadata = Arc::new(metadata);
    let peer_tasks_handles = peers.iter().map(|peer: &String| {
        // let metadata = metadata.clone();
        let peer = peer.clone();
        let metadata = metadata.clone();

        tokio::spawn(async move {
            let (_, stream) = handshake::get_handshake(&metadata, &peer).await;
            let bitmap = download_piece::get_bitfield(&stream).await;

            PeerTask {
                stream,
                bitmap,
                pieces: Vec::new(),
                metadata,
            }
        })
    });

    let mut peer_tasks = Vec::with_capacity(peer_tasks_handles.len());
    for task in peer_tasks_handles {
        peer_tasks.push(task.await.expect("Failed to get peer stream"));
    }

    let mut not_found_counter = 0;
    for piece in piece_hashes.iter().enumerate() {
        let (piece_idx, piece_hash) = piece;
        let peer_idx = piece_idx % peer_tasks.len();

        if peer_tasks[peer_idx].bitmap[piece_idx] {
            peer_tasks[peer_idx].pieces.push(Piece {
                index: piece_idx as u32,
                hash: piece_hash.clone(),
            });
        } else {
            not_found_counter += 1;
            if not_found_counter >= peer_tasks.len() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No peers have this piece",
                ));
            }
        }
    }

    let file = Arc::new(RwLock::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(output_path)
            .await?,
    ));

    let peer_tasks: Vec<Arc<PeerTask>> = peer_tasks.into_iter().fold(Vec::new(), |mut acc, p_t| {
        acc.push(Arc::new(p_t));
        acc
    });

    let peer_tasks_handles =
        peer_tasks
            .iter()
            .map(|peer_task| -> JoinHandle<Result<(), io::Error>> {
                let file = file.clone();
                let peer_task = peer_task.clone();
                let piece_hashes = piece_hashes.clone();

                tokio::spawn(async move {
                    //? Send interested message
                    peer_task
                        .stream
                        .write()
                        .await
                        .write_all(&[0, 0, 0, 1, 2])
                        .await?;

                    //? Unchoke message
                    let message = download_piece::receive_message(&peer_task.stream).await?;
                    assert!(message.id == 1);

                    for piece in peer_task.pieces.iter() {
                        let piece_index = piece.index;
                        let pieces_count = piece_hashes.len() as u32;
                        let piece_hash = &piece.hash.clone();

                        let metadata = peer_task.metadata.read().await;
                        let piece_length = if piece_index == pieces_count - 1 {
                            metadata.info.length - (piece_index * metadata.info.piece_length)
                        } else {
                            metadata.info.piece_length
                        };
                        let piece_position = piece_index * metadata.info.piece_length;

                        //? Piece blocks messages to send
                        let piece_blocks_messages =
                            download_piece::get_piece_blocks_messages(piece_index, piece_length);

                        //? Received piece blocks
                        let piece_blocks = download_piece::receive_piece_blocks(
                            &peer_task.stream,
                            piece_blocks_messages,
                        )
                        .await;

                        let piece = download_piece::combine_blocks_into_piece(
                            piece_blocks,
                            piece_length,
                            piece_index,
                            piece_hash,
                        );

                        write_at_position(file.clone(), piece_position as u64, &piece).await?;
                    }

                    Ok(())
                })
            });

    let mut peer_tasks = Vec::with_capacity(peer_tasks_handles.len());
    for task in peer_tasks_handles {
        peer_tasks.push(task.await.expect("Failed to get peer stream"));
    }

    Ok(())
}

struct PeerTask {
    stream: RwLock<TcpStream>,
    bitmap: Vec<bool>,
    pieces: Vec<Piece>,
    metadata: Arc<RwLock<info::Metadata>>,
}

struct Piece {
    index: u32,
    hash: String,
}

#[allow(dead_code)]
async fn write_at_position(
    file: Arc<RwLock<tokio::fs::File>>,
    position: u64,
    data: &[u8],
) -> io::Result<()> {
    let mut file = file.write().await;
    file.seek(io::SeekFrom::Start(position)).await?;
    file.write_all(data).await?;
    Ok(())
}
