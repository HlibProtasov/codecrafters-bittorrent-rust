pub mod tracker;
pub mod peer;
pub mod torrent;
pub mod downloaded;
pub mod piece;

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
            peer: String,
        },
        #[clap(name = "download_piece")]
        DownloadPiece
        {
            output: PathBuf,
            torrent: PathBuf,
            piece: usize,
        },
        Download
        {
            torrent: PathBuf,
            output: PathBuf,
        },
    }
}

pub mod torrent_executor
{
    use std::net::SocketAddrV4;
    use std::str::FromStr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use crate::cli::Commands;
    use crate::peer::{Handshake, Message, MessageFramer, MessageTag, PeerRequest, PieceMessage};
    use crate::tracker::{TrackerResponse, url_encode};
    use crate::tracker::TrackerRequest;
    use anyhow::Context;
    use tokio::net::TcpStream;
    use crate::decoder::decode_bencoded_value;
    use futures_util::stream::StreamExt;
    use futures_util::SinkExt;
    use sha1::{Sha1, Digest};
    use crate::peer::MessageTag::{Request};
    use crate::torrent::{Keys, Torrent};

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
                    }
                Commands::DownloadPiece { torrent, output, piece } =>
                    {
                        let torrent = Torrent::try_from(&torrent).context("Deriving torrent")?;
                        let tracker_response = Self::get_peers(&torrent).await?;

                        let peer = tracker_response.peers.0[0]; // Can connect to all peers or to randome one
                        let peer = Self::handshake(torrent.info_hash()?, peer).await?;
                        let mut framed = tokio_util::codec::Framed::new(peer, MessageFramer);
                        let msg = framed.next().await.expect("Peer always sends first message")
                            .context("Deriving message")?;
                        assert_eq!(msg.tag, MessageTag::Bitfield, "First message should has bitfield tag");
                        framed.send(
                            Message
                            {
                                tag: MessageTag::Interested,
                                payload: vec![],
                            }
                        ).await.context("Sending 'interesting' message")?;
                        let msg = framed.next().await.expect("Peer always sends first message")
                            .context("Deriving message")?;

                        assert_eq!(msg.tag, MessageTag::UnChoke, "Waiting for 'unchoke' message");
                        assert!(torrent.info.pieces.0.len() > piece, "Can't get peice hash");

                        let peice_hash = torrent.info.pieces.0[piece];
                        let piece_size = match (piece, torrent.info.keys)
                        {
                            (p, Keys::SingleFile { length }) if p == torrent.info.pieces.0.len() + 1 => {
                                length % torrent.info.length
                            }
                            (_, Keys::SingleFile { length: _ }) => torrent.info.length,

                            _ => unimplemented!()
                        };
                        let nblocks = (piece_size + 2 ^ 14 - 1) / 2 ^ 14;
                        let mut pieces: Vec<u8> = Vec::with_capacity(nblocks);
                        for block in 0..nblocks
                        {
                            let block_size = if block == nblocks {
                                piece_size % 2 ^ 14
                            } else {
                                2 ^ 14
                            };
                            let request = PeerRequest::new(piece as u32, (block * 2 ^ 14) as u32, block_size as u32);
                            // TODO! add safe casting

                            framed.send(
                                Message
                                {
                                    tag: Request,
                                    payload: request.to_bytes().to_vec(),
                                }
                            ).await.
                                with_context(||
                                    format!("request with index: {}, len: {}, begin: {}",
                                            request.index(), request.length(), request.begin()
                                    )
                                )?;
                            let msg = framed.next().await.expect("Peer always sends first message")
                                .context("Deriving piece message")?;
                            assert_eq!(msg.tag, MessageTag::Piece, "Waiting for the 'Piece message'");
                            assert!(!msg.payload.is_empty(), "Shuldn't be empty");
                            let piece = PieceMessage::from_bytes(msg.payload.as_slice());
                            pieces.extend(piece.block());
                        }
                        let mut sha = Sha1::new();
                        sha.update(pieces);
                        let hash = sha.finalize();
                        println!("{:?}", hash);
                    }
                Commands::Download { torrent, output } => {
                    // let torrent = Torrent::read(output)?;
                    // torrent.print_tree();
                    // let files = torrent.download_all().await?;
                    // tokio::fs::write(
                    //     &output, files.iter().next().expect("always one file").bytes(),
                    // ).await?;
                    // torrent.download_some(vec![("/foo.txt", output)]).await?;
                    // torrent.download_single(output).await?;
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
            let mut handshake = Handshake::new(hash_info);
            let mut peer = tokio::net::TcpStream::connect(socket).await.context("Creating connection to peer")?;
            let handshake_bytes = handshake.to_bytes_mut();
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