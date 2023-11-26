pub mod tracker;
pub mod peer;
pub mod cli
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
        Peers
        {
            torrent: PathBuf
        },
        Handshake
        {
            torrent: PathBuf,
            peer: String
        }
    }
}


pub mod hashes
{
    use std::fmt;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

    impl Serialize for Hashes
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer
        {
            let single_slice = self.0.concat();
            serializer.serialize_bytes(&single_slice)
        }
    }
}


use std::path::PathBuf;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha1::{Sha1,Digest};
use crate::hashes::Hashes;


#[derive(Deserialize, Debug, Clone)]
pub struct Torrent
{
    pub announce: String,
    pub info: Info,
}
impl Torrent
{
    pub fn info_hash(&self) -> anyhow::Result<[u8; 20]>
    {
        let re_encoded = serde_bencode::to_bytes(&self.info)?;

        let mut hash = Sha1::new();
        hash.update(re_encoded);
       Ok( hash.finalize().into() )
    }
}
impl TryFrom<&PathBuf> for Torrent
{
    type Error = anyhow::Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let f = std::fs::read(value).context("Read torrent file")?;
        serde_bencode::from_bytes(&f).context("Parse torrent file")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info
{
    pub name: String,
    pub length: usize,
    #[serde(rename(deserialize = "piece length"))]
    pub piece_length: usize,
    pub pieces: Hashes,
    #[serde(flatten)]
    pub keys: Keys,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Keys
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct File
{
    pub length: usize,
    pub path: Vec<String>,
}

pub mod decoder {
    use serde_json::Value;

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
    pub fn decode_bencoded_value(encoded_value: &str) -> anyhow::Result<Value> {
        // If encoded_value starts with a digit, it's a number

        let (value, _) = decode(encoded_value)?;
        Ok(value)
    }
}