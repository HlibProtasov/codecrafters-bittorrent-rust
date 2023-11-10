use serde_json;
use std::env;
use std::str::Chars;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
    if encoded_value.is_empty()
    {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
    match &encoded_value.chars().next().unwrap() {
        'i' => {
            let number = encoded_value[1..encoded_value.len() - 1].parse::<i64>().unwrap();
            return serde_json::Value::Number(number.into());
        }
        '0'..='9' =>
            {
                if let Some((len, other)) = encoded_value.split_once(':')
                {
                    if let Ok(number) = len.parse::<usize>() {

                        // Example: "5:hello" -> "hello"
                        let string = &other[..number];
                        return serde_json::Value::String(string.to_owned());
                    }
                }
            }
        _ => {}
    }


    panic!("Unhandled encoded value: {}", encoded_value)
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        eprintln!("unknown command: {}", args[1])
    }
}
