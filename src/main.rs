use std::env;

mod decode;
mod info;

use decode::decode_serde_bencode;
use info::get_info;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: your_program decode '<encoded_value>'");
        return;
    }
    let command = &args[1];
    let parameter = &args[2];

    match command.as_str() {
        "decode" => match decode_serde_bencode(parameter.as_bytes()) {
            Ok(decoded) => println!("{}", decoded),
            Err(e) => println!("Error: {}", e),
        },
        "info" => println!("{}", get_info(parameter)),
        _ => println!("unknown command: {}", command),
    }
}
