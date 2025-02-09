use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use crate::info::Metadata;

pub async fn get_handshake(metadata: &RwLock<Metadata>, peer: &str) -> (String, RwLock<TcpStream>) {
    let handshake = construct_handshake(metadata).await;

    let mut stream = TcpStream::connect(peer)
        .await
        .expect("Could not connect to server");
    stream
        .write_all(&handshake)
        .await
        .expect("Failed to send message");

    let mut buffer = [0; 68];
    stream
        .read_exact(&mut buffer)
        .await
        .expect("Failed to read from server");

    (
        buffer[48..]
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<String>>()
            .join(""),
        RwLock::new(stream),
    )
}

async fn construct_handshake(metadata: &RwLock<Metadata>) -> Vec<u8> {
    let mut handshake = Vec::new();
    handshake.push(19);
    handshake.extend(b"BitTorrent protocol");
    handshake.extend(vec![0; 8]);
    handshake.extend(metadata.write().await.info.get_hash());
    handshake.extend(b"21372137696921372137");
    handshake
}
