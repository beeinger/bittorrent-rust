use clap::Parser;
use serde_bencode::from_bytes;
use tokio::sync::RwLock;

mod cli;
mod decode;
mod download;
mod download_piece;
mod handshake;
mod info;
mod peers;

use cli::Cli;
use decode::BencodeValue;
use download::download;
use download_piece::download_piece;
use handshake::get_handshake;
use info::get_info;
use peers::get_peers;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(cli::Commands::Decode { bencoded_value }) => println!(
            "{}",
            from_bytes::<BencodeValue>(bencoded_value.as_bytes())
                .expect("Invalid bencoded value")
                .to_json()
        ),
        Some(cli::Commands::Info { torrent_file }) => println!("{}", get_info(&torrent_file)),
        Some(cli::Commands::Peers { torrent_file }) => {
            println!("{}", {
                let metadata = RwLock::new(info::get_info(&torrent_file));
                get_peers(&metadata).await.join("\n")
            })
        }
        Some(cli::Commands::Handshake { torrent_file, peer }) => {
            println!("Peer ID: {}", {
                let metadata = RwLock::new(info::get_info(&torrent_file));
                get_handshake(&metadata, &peer).await.0
            })
        }
        Some(cli::Commands::DownloadPiece {
            torrent_file,
            piece_index,
            output_path,
        }) => {
            download_piece(&torrent_file, piece_index, &output_path)
                .await
                .expect("Failed to download piece");
            println!(
                "Piece {} downloaded to {}.",
                piece_index,
                output_path.to_str().unwrap()
            );
        }
        Some(cli::Commands::Download {
            torrent_file,
            output_path,
        }) => {
            download(&torrent_file, &output_path)
                .await
                .expect("Failed to download");
            println!(
                "Downloaded {} to {}.",
                torrent_file.to_str().unwrap(),
                output_path.to_str().unwrap()
            );
        }
        None => rdza::rdza! {kurwa!("Podaj jakąś komendę debulu...")},
    }
}
