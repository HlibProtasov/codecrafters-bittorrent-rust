use serde_json;
use std::env;

use clap::builder::TypedValueParser;
use serde_json::Value;

enum ParseTy
{
    Num,
    Str,
}


// Available if you need it!
// use serde_bencode
fn decode(encoded_value: &str) -> (Value, &str)
{
    // i53e
    match &encoded_value.chars().next() {
        Some('i') =>
            {
                let mut rest_str = "";
                if let Some(number) = encoded_value
                    .split_once('e')
                    .and_then(|(num, rest)|
                        {
                            rest_str = rest;

                            num[1..].parse::<i64>().ok()
                        })
                {
                    return (number.into(), rest_str);
                }
            }
        Some('0'..='9') =>
            {
                if let Some((len, other)) = encoded_value.split_once(':')
                {
                    if let Ok(number) = len.parse::<usize>() {

                        // Example: "5:hello" -> "hello"
                        let string = &other[..number];
                        return (string.into(), &other[number..]);
                    }
                }
            }

        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value)
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> Value {
    // If encoded_value starts with a digit, it's a number

    match encoded_value.chars().next() {
        Some('l') => {
            let mut result = &encoded_value[1..];
            let mut vec = Vec::new();
            loop {
                let (value, rest) = decode(result);
                vec.push(value);
                if rest.is_empty() || rest.chars().next().unwrap() == 'e'
                {
                    return vec.into();
                }
                result = rest;
            }
        }
        Some(_) => {
            let (value, _) = decode(encoded_value);
            return value;
        }
        _ => {
            panic!("Unhandled encoded value: {}", encoded_value)
        }
    }
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
