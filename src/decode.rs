use serde::{Deserialize, Serialize};
use serde_bencode::value::Value as ExternalBencodeValue;
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize)]
pub struct BencodeValue(pub ExternalBencodeValue);

impl BencodeValue {
    pub fn to_json(&self) -> Value {
        match &self.0 {
            ExternalBencodeValue::Bytes(bytes) => {
                Value::String(String::from_utf8_lossy(bytes).to_string())
            }
            ExternalBencodeValue::Int(i) => Value::Number((*i).into()),
            ExternalBencodeValue::List(list) => {
                let vec: Vec<Value> = list
                    .iter()
                    .map(|item| BencodeValue(item.clone()).to_json())
                    .collect();
                Value::Array(vec)
            }
            ExternalBencodeValue::Dict(dict) => {
                let mut map = serde_json::map::Map::new();
                for (key, value) in dict {
                    map.insert(
                        String::from_utf8_lossy(key).to_string(),
                        BencodeValue(value.clone()).to_json(),
                    );
                }
                Value::Object(map)
            }
        }
    }
}

//? old code
#[allow(dead_code)]
pub fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    let next = encoded_value.chars().next().unwrap();
    if next == 'e' {
        (serde_json::Value::Null, &encoded_value[1..])
    } else if next == 'd' {
        let mut remaining = &encoded_value[1..encoded_value.len()];
        let mut dict = Map::new();

        'dict: while !remaining.is_empty() {
            let mut key: Value = Value::Null;

            for _ in 0..2 {
                let (decoded_value, new_remaining) = decode_bencoded_value(remaining);
                remaining = new_remaining;

                if decoded_value == serde_json::Value::Null {
                    break 'dict;
                }

                match key {
                    Value::String(ref str_key) => {
                        dict.insert(str_key.as_str().into(), decoded_value);
                    }
                    _ => {
                        key = decoded_value;
                        continue;
                    }
                }
            }
        }

        (serde_json::Value::Object(dict), &remaining)
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
    } else if next.is_ascii_digit() {
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
