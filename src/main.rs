use clap::Parser;
use handshake::get_handshake;
use serde_bencode::from_bytes;

mod cli;
mod decode;
mod handshake;
mod info;
mod peers;

use cli::Cli;
use decode::BencodeValue;
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
        Some(cli::Commands::Info { torrent_file }) => println!("{}", get_info(torrent_file)),
        Some(cli::Commands::Peers { torrent_file }) => {
            println!("{}", get_peers(torrent_file).await.join("\n"))
        }
        Some(cli::Commands::Handshake { torrent_file, peer }) => {
            println!("Peer ID: {}", get_handshake(torrent_file, &peer).await)
        }
        Some(cli::Commands::DownloadPiece {
            torrent_file,
            piece_index,
            output_path,
        }) => println!(
            "{}: Piece {} downloaded to {}.",
            torrent_file.to_str().unwrap(),
            piece_index,
            output_path.to_str().unwrap()
        ),
        None => println!("No command provided"),
    }
}
