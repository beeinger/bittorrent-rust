use serde_json;
use std::env;

// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    let next = encoded_value.chars().next().unwrap();
    if next == 'e' {
        (serde_json::Value::Null, &encoded_value[1..])
    } else if next == 'l' {
        let mut remaining = &encoded_value[1..encoded_value.len()];
        let mut list = Vec::new();

        while !remaining.is_empty() {
            let (decoded_value, new_remaining) = decode_bencoded_value(remaining);
            remaining = new_remaining;
            if decoded_value == serde_json::Value::Null {
                break;
            }
            list.push(decoded_value);
        }

        (serde_json::Value::Array(list), &remaining)
    } else if next == 'i' {
        let e_index = encoded_value.find('e').unwrap();
        let number_string = &encoded_value[1..e_index];
        let number = number_string.parse::<i64>().unwrap();

        (
            serde_json::Value::Number(number.into()),
            &encoded_value[e_index + 1..],
        )
    } else if next.is_digit(10) {
        let colon_index = encoded_value.find(':').unwrap();
        let number_string = &encoded_value[..colon_index];
        let number = number_string.parse::<i64>().unwrap();
        let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];

        (
            serde_json::Value::String(string.to_string()),
            &encoded_value[colon_index + 1 + number as usize..],
        )
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

//? Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.0.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
