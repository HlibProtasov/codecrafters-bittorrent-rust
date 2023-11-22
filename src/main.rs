use anyhow::Context;
use serde_json;
use clap::Args;
use clap::builder::TypedValueParser;
use serde_json::Value;
use serde::Deserialize;
use hashes::Hashes;
use cli::Cli;
use clap::Parser;
use cli::Commands::
{
    Decode,
    Info as CInfo,
};

#[derive(Deserialize, Debug, Clone)]
struct Torrent
{
    announce: String,
    info: Info,
}

#[derive(Debug, Deserialize, Clone)]
struct Info
{
    name: String,
    length: usize,
    #[serde(rename(deserialize = "piece length"))]
    piece_length: usize,
    pieces: Hashes,
    #[serde(flatten)]
    keys: Keys,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Keys
{
    SingleFile
    {
        length: usize
    },
    MultiFile
    {
        file: Vec<File>
    },
}

#[derive(Deserialize, Debug, Clone)]
struct File
{
    length: usize,
    path: Vec<String>,
}


// Available if you need it!
// use serde_bencode

fn decode(encoded_value: &str) -> anyhow::Result<(Value, &str)>
{
    match encoded_value.chars().next() {
        Some('l') => {
            let mut result = &encoded_value[1..];
            let mut vec = Vec::new();
            loop {
                let (value, rest) = decode(result)?;
                vec.push(value);
                if rest.is_empty() || rest.chars().next().expect("Rest is not empty.") == 'e'
                {
                    return Ok((vec.into(), result));
                }
                result = rest;
            }
        }
        Some('d') => {
            // d3:"key":value
            let mut map = serde_json::map::Map::new();
            let &(_, mut encoded) = &encoded_value.split_at(1);

            loop {
                let (key, rest) = decode(encoded)?;
                if let Value::String(k) = key
                {
                    let (value, rest) = decode(rest)?;
                    map.insert(k, value);
                    if rest.is_empty() || rest.chars().next().unwrap() == 'e'
                    {
                        return Ok((map.into(), rest));
                    }
                    encoded = rest;
                } else {
                    panic!("Unhandled encoded value: {}", encoded_value)
                }
            }
        }
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
                    return Ok((number.into(), rest_str));
                }
            }
        Some('0'..='9') =>
            {
                if let Some((len, other)) = encoded_value.split_once(':')
                {
                    if let Ok(number) = len.parse::<usize>() {

                        // Example: "5:hello" -> "hello"
                        let string = &other[..number];
                        return Ok((string.into(), &other[number..]));
                    }
                }
            }

        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value)
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> anyhow::Result<Value> {
    // If encoded_value starts with a digit, it's a number

    let (value, _) = decode(encoded_value)?;
    Ok(value)
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() -> anyhow::Result<()>{
    let cli = Cli::parse();
    match &cli.command
    {
        Decode { value } =>
            {
                let decoded_value = decode_bencoded_value(&value)?;
                println!("{}", decoded_value.to_string());
            }
        CInfo { torrent } =>
            {
                let mut f = std::fs::read(torrent).context("Read torrent file")?;
                let t: Torrent = serde_bencode::from_bytes(&f).context("Parse torrent file")?;
                println!("{:#?}", t);

            }

    }
    Ok(())
}


mod hashes
{
    use std::fmt;
    use serde::de::{Error, Visitor};
    use serde::{
        Deserialize,
        Deserializer,
    };

    #[derive(Debug, Clone)]
    pub struct Hashes(pub Vec<[u8; 20]>);

    struct HashesVisitor;

    impl<'de> Visitor<'de> for HashesVisitor {
        type Value = Hashes;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a byte string whose length is a multiple of 20")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: Error {
            if v.len() % 20 != 0
            {
                return Err(E::custom(format!("lenght is: {} .", v.len())));
            }

            Ok(
                Hashes(
                    v.chunks_exact(20).
                        map(|slice_20| slice_20.try_into().expect("guaranty to be 20")).
                        collect()
                )
            )
        }
        fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E> where E: Error {
            todo!()
        }
    }

    impl<'de> Deserialize<'de> for Hashes {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>
        {
            deserializer.deserialize_bytes(HashesVisitor)
        }
    }
}


mod cli
{
    use std::path::PathBuf;
    use clap::{
        Parser,
        Subcommand,
    };


    #[derive(Parser, Debug, Clone)]
    pub struct Cli
    {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug, Clone)]
    pub enum Commands
    {
        Decode
        {
            value: String,
        },
        Info
        {
            torrent: PathBuf
        },
    }
}