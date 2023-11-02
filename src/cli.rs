use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Decodes bencoded values
    Decode {
        /// A bencoded value to decode
        bencoded_value: String,
    },
    /// Gets info about a torrent file
    Info {
        /// A torrent file to decode
        torrent_file: PathBuf,
    },
    /// Gets peers from a torrent file
    Peers {
        /// A torrent file to decode
        torrent_file: PathBuf,
    },
    /// Gets the peer ID from a handshake
    Handshake {
        /// A torrent file to decode
        torrent_file: PathBuf,
        /// Peer IP address and port
        peer: String,
    },
    /// Downloads a piece from a torrent file
    #[command(name = "download_piece")]
    DownloadPiece {
        /// A torrent file to decode
        torrent_file: PathBuf,
        /// Piece index
        piece_index: usize,
        /// Output path
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: PathBuf,
    },
    Download {
        /// A torrent file to decode
        torrent_file: PathBuf,
        /// Output path
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: PathBuf,
    },
}
