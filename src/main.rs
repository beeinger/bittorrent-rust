use handshake::get_handshake;
use serde_bencode::{from_bytes, value::Value};
use std::env;

mod decode;
mod handshake;
mod info;
mod peers;

use decode::convert_bencode_decode_result_to_json_values;
use info::get_info;
use peers::get_peers;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        println!("Usage: your_program decode '<encoded_value>'");
        return;
    }
    let command = &args[1];
    let parameter1 = &args[2];
    let parameter2 = if args.len() == 4 { &args[3] } else { "" };

    match command.as_str() {
        "decode" => match from_bytes::<Value>(parameter1.as_bytes()) {
            Ok(decoded_value) => println!(
                "{}",
                convert_bencode_decode_result_to_json_values(&decoded_value)
            ),
            Err(e) => println!("Error: {}", e),
        },
        "info" => println!("{}", get_info(parameter1)),
        "peers" => println!("{}", get_peers(parameter1).await.join("\n")),
        "handshake" => println!("Peer ID: {}", get_handshake(parameter1, parameter2).await),
        _ => println!("unknown command: {}", command),
    }
}
