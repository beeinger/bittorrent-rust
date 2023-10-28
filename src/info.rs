use std::fs;

use crate::decode::decode_serde_bencode;
use std::fmt::Display;

pub struct Metadata {
    announce: String,
    info: Info,
}

#[allow(dead_code)]
pub struct Info {
    name: String,
    piece_length: u32,
    pieces: Vec<u8>,
    length: u32,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tracker URL: {}\nLength: {}",
            self.announce, self.info.length
        )
    }
}

pub fn get_info(path: &str) -> Metadata {
    let contents: Vec<u8> = fs::read(path).unwrap();
    let decoded: serde_json::Value = decode_serde_bencode(&contents).unwrap();

    Metadata {
        announce: decoded["announce"].as_str().unwrap().to_string(),
        info: Info {
            name: decoded["info"]["name"].as_str().unwrap().to_string(),
            piece_length: decoded["info"]["piece length"].as_u64().unwrap() as u32,
            pieces: decoded["info"]["pieces"]
                .as_str()
                .unwrap()
                .to_string()
                .into_bytes(),
            length: decoded["info"]["length"].as_u64().unwrap() as u32,
        },
    }
}
