use serde::{Deserialize, Serialize};
use serde_bencode::{from_bytes, to_bytes};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};
use std::fs;

use std::fmt::Display;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    announce: String,
    info: Info,
}

#[allow(dead_code)]
#[derive(Serialize, Debug, Deserialize)]
pub struct Info {
    name: String,
    #[serde(rename = "piece length")]
    piece_length: u32,
    pieces: ByteBuf,
    length: u32,
}

impl Info {
    pub fn hash(&self) -> String {
        let bencoded_info = to_bytes(&self).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(bencoded_info);
        let hash: [u8; 20] = hasher.finalize().into();
        hex::encode(hash)
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tracker URL: {}\nLength: {}\nInfo Hash: {}",
            self.announce,
            self.info.length,
            self.info.hash()
        )
    }
}

pub fn get_info(path: &str) -> Metadata {
    let contents: Vec<u8> = fs::read(path).unwrap();
    from_bytes::<Metadata>(&contents).unwrap()
}
