pub mod tracker;
pub mod peer;

pub mod cli
{
    use std::path::PathBuf;
    use clap::{
        Parser,
        Subcommand,
    };
    use crate::peer::MessageFramer;


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
            peer: String,
        },
        DownloadPiece
        {
            output: PathBuf,
            torrent: PathBuf,
            piece: usize
        }
    }
}

pub mod TorrentExecutor
{
    use std::net::SocketAddrV4;
    use std::str::FromStr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use crate::cli::Commands;
    use crate::peer::{Handshake, MessageFramer};
    use crate::{Keys, Torrent};
    use crate::tracker::{TrackerResponse, url_encode};
    use crate::tracker::TrackerRequest;
    use anyhow::Context;
    use tokio::net::TcpStream;
    use crate::decoder::decode_bencoded_value;
    use futures_util::stream::StreamExt;

    pub struct TorrentExecutor;

    impl TorrentExecutor
    {
        pub async fn execute(command: Commands) -> anyhow::Result<()>
        {
            match command
            {
                Commands::Decode { value } =>
                    {
                        let decoded_value = decode_bencoded_value(&value)?;
                        println!("{}", decoded_value.to_string());
                    }
                Commands::Info { torrent } =>
                    {
                        let t = Torrent::try_from(&torrent)?;
                        println!("Tracked url: {}", t.announce);
                        let length = if let Keys::SingleFile { length } = t.info.keys
                        {
                            println!("Length {length}");
                            length
                        } else {
                            todo!()
                        };
                        let hash = t.info_hash()?;

                        println!("Info hash: {}", hex::encode(hash));
                        println!("Piece length: {}", t.info.piece_length);
                        println!("Piece hashes: ");
                        t.info.pieces.0.iter().for_each(|data| println!("{}", hex::encode(data)));
                    }
                Commands::Peers { torrent } =>
                    {
                        let t = Torrent::try_from(&torrent)?;

                        let peers = TorrentExecutor::get_peers(&t).await.context("Getting peers")?;
                        for peer in peers.peers.0 {
                            println!("{}:{}", peer.ip(), peer.port());
                        }
                    }

                Commands::Handshake { torrent, peer } =>
                    {
                        let t = Torrent::try_from(&torrent)?;
                        println!("Tracked url: {}", &t.announce);

                        let hash = t.info_hash()?;
                        let socket = SocketAddrV4::from_str(&peer).context("Deriving socket")?;
                        TorrentExecutor::handshake(hash, socket).await.context("Making handshake")?;
                    },
                Commands::DownloadPiece { torrent, output, piece } =>
                    {
                        let torrent = Torrent::try_from(&torrent).context("Deriving torrent")?;
                        let tracker_response = Self::get_peers(&torrent).await?;

                        let peer = tracker_response.peers.0[0]; // Can connect to all peers or to randome one
                        let peer = Self::handshake(torrent.info_hash()?,peer).await?;
                        let mut framed = tokio_util::codec::Framed::new(peer, MessageFramer);
                        let msg = framed.next().await.context("Waiting for message")?
                            .context("Deriving message")?;


                    }
                _ => { unimplemented!() }
            }
            Ok(())
        }
        async fn get_peers(torrent: &Torrent) -> anyhow::Result<TrackerResponse>
        {
            if let Keys::SingleFile { length } = torrent.info.keys
            {
                let tracker_request = TrackerRequest::new(String::from("011112012313"), length);

                let url_params = serde_urlencoded::to_string(tracker_request).
                    context("URL-tracker params")?;
                let info_hash = torrent.info_hash()?;
                let tracker_url = format!("{}?{}&info_hash={}", torrent.announce, url_params, &url_encode(&info_hash));


                let response = reqwest::get(tracker_url).await.context("Fetch Tracker")?;
                let response: TrackerResponse = serde_json::from_slice(&response.bytes().await
                    .context("Fetch tracker response")?)
                    .context("Serialising response bytes")?;

                Ok(response)
            } else {
                unreachable!()
            }
        }
        async fn handshake(hash_info: [u8; 20], socket: SocketAddrV4) -> anyhow::Result<TcpStream>
        {
            let handshake = Handshake::new(hash_info);
            let mut peer = tokio::net::TcpStream::connect(socket).await.context("Creating connection to peer")?;
            let handshake_bytes = handshake.to_bytes();
            peer.write_all(&handshake_bytes).await.context("Writing to peer")?;

            let mut buffer = Vec::new();
            peer.read_to_end(&mut buffer).await?;
            let response_handshake = Handshake::from_bytes(&buffer).context("Getting back handshake")?;
            println!("Handshake: {:?}", response_handshake);
            Ok(peer)
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
use sha1::{Sha1, Digest};
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
        Ok(hash.finalize().into())
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