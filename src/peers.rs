use crate::info;
use serde::{self, Deserialize, Serialize};
use serde_bencode::from_bytes;
use serde_bytes::ByteBuf;

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
                    (((curr_peer[4] as u16) << 8) + curr_peer[5] as u16).to_string()
                ));
                curr_peer = Vec::new();
            }
        }

        peers
    }
}

pub async fn get_peers(path: &str) -> Vec<String> {
    let metadata = info::get_info(path);

    let dicover_peers_query = DiscoverPeersQuery {
        info_hash: urlencode(&metadata.info.get_hash()),
        peer_id: "21372137696921372137".to_string(),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: metadata.info.length,
        compact: 1,
    };

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

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}
