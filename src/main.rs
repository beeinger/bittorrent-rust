use std::env;

mod decode;

use decode::decode_serde_bencode;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: your_program decode '<encoded_value>'");
        return;
    }
    let command = &args[1];
    let parameter = &args[2];

    match command.as_str() {
        "decode" => match decode_serde_bencode(parameter) {
            Ok(decoded) => println!("{}", decoded),
            Err(e) => println!("Error: {}", e),
        },
        "info" => println!("info"),
        _ => println!("unknown command: {}", command),
    }
}
