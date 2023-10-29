use serde::{Deserialize, Serialize};
use serde_bencode::{from_bytes, to_bytes};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};
use std::fs;

use std::fmt::Display;
use std::path::PathBuf;

pub fn get_info(path: PathBuf) -> Metadata {
    let contents: Vec<u8> = fs::read(path).unwrap();
    from_bytes::<Metadata>(&contents).unwrap()
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub announce: String,
    pub info: Info,
}

#[allow(dead_code)]
#[derive(Serialize, Debug, Deserialize)]
pub struct Info {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    pub pieces: ByteBuf,
    pub length: u32,
}

impl Info {
    pub fn get_hash(&self) -> [u8; 20] {
        let bencoded_info = to_bytes(&self).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(bencoded_info);
        let hash: [u8; 20] = hasher.finalize().into();
        hash
    }

    pub fn get_hex_hash(&self) -> String {
        hex::encode(self.get_hash())
    }

    pub fn get_piece_hashes(&self) -> Vec<String> {
        let mut piece_hashes: Vec<String> = Vec::new();
        let mut curr_piece: Vec<u8> = Vec::new();

        for (i, byte) in self.pieces.iter().enumerate() {
            curr_piece.push(byte.to_owned());
            if (i + 1) % 20 == 0 {
                piece_hashes.push(hex::encode(curr_piece));
                curr_piece = Vec::new();
            }
        }

        piece_hashes
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tracker URL: {}\nLength: {}\nInfo Hash: {}\nPiece Length: {}\nPiece Hashes:\n{}",
            self.announce,
            self.info.length,
            self.info.get_hex_hash(),
            self.info.piece_length,
            self.info.get_piece_hashes().join("\n")
        )
    }
}
