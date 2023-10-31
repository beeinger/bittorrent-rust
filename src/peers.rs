use crate::info::Metadata;
use serde::{self, Deserialize, Serialize};
use serde_bencode::from_bytes;
use serde_bytes::ByteBuf;

pub async fn get_peers(metadata: Metadata) -> Vec<String> {
    let dicover_peers_query = DiscoverPeersQuery::new(
        "21372137696921372137".to_string(),
        6881,
        0,
        0,
        metadata.info.length,
        1,
        metadata.info.get_hash(),
    );

    let res = reqwest::get(format!(
        "{}?{}",
        metadata.announce,
        dicover_peers_query.get_query_string()
    ))
    .await
    .unwrap();

    let bytes = res.bytes().await.unwrap();
    let decoded = from_bytes::<DiscoverPeersResponse>(&bytes).unwrap();

    decoded.get_peers()
}

#[derive(Serialize, Deserialize)]
struct DiscoverPeersQuery {
    peer_id: String,
    port: u32,
    uploaded: u32,
    downloaded: u32,
    left: u32,
    compact: u32,
    info_hash: String,
}

impl DiscoverPeersQuery {
    pub fn new(
        peer_id: String,
        port: u32,
        uploaded: u32,
        downloaded: u32,
        left: u32,
        compact: u32,
        info_hash: [u8; 20],
    ) -> Self {
        Self {
            peer_id,
            port,
            uploaded,
            downloaded,
            left,
            compact,
            info_hash: urlencode(&info_hash),
        }
    }

    pub fn get_query_string(&self) -> String {
        serde_urlencoded::to_string(self)
            .unwrap()
            .split("info_hash=")
            .next()
            .unwrap()
            .to_owned()
            + "info_hash="
            + &self.info_hash
    }
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode([byte]));
    }
    encoded
}

#[derive(Serialize, Deserialize, Debug)]
struct DiscoverPeersResponse {
    complete: u32,
    incomplete: u32,
    interval: u32,
    #[serde(rename = "min interval")]
    min_interval: u32,
    peers: ByteBuf,
}

impl DiscoverPeersResponse {
    pub fn get_peers(&self) -> Vec<String> {
        let mut peers: Vec<String> = Vec::new();
        let mut curr_peer: Vec<u8> = Vec::new();

        for (i, byte) in self.peers.iter().enumerate() {
            curr_peer.push(byte.to_owned());
            if (i + 1) % 6 == 0 {
                peers.push(format!(
                    "{}.{}.{}.{}:{}",
                    curr_peer[0],
                    curr_peer[1],
                    curr_peer[2],
                    curr_peer[3],
                    (((curr_peer[4] as u16) << 8) + curr_peer[5] as u16)
                ));
                curr_peer = Vec::new();
            }
        }

        peers
    }
}
