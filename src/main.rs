use serde_bencode::{from_bytes, value::Value};
use std::env;

mod decode;
mod info;
mod peers;

use decode::convert_bencode_decode_result_to_json_values;
use info::get_info;
use peers::get_peers;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: your_program decode '<encoded_value>'");
        return;
    }
    let command = &args[1];
    let parameter = &args[2];

    match command.as_str() {
        "decode" => match from_bytes::<Value>(parameter.as_bytes()) {
            Ok(decoded_value) => println!(
                "{}",
                convert_bencode_decode_result_to_json_values(&decoded_value)
            ),
            Err(e) => println!("Error: {}", e),
        },
        "info" => println!("{}", get_info(parameter)),
        "peers" => println!("{}", get_peers(parameter).await.join("\n")),
        _ => println!("unknown command: {}", command),
    }
}
