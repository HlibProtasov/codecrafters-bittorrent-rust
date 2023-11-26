use std::net::SocketAddrV4;
use std::str::FromStr;
use anyhow::Context;

use ::bittorrent_starter_rust::{
    decoder::decode_bencoded_value,
    cli::{
        Commands::{
            Decode,
            Info as CInfo,
            Peers,
            Handshake,
        },
        Cli,
    },
    Torrent,
    Keys,
};
use clap::Parser;

use sha1::{Sha1, Digest};
use tokio::io::AsyncWriteExt;
use tokio::io::{self, AsyncReadExt};
use bittorrent_starter_rust::peer::Handshake as HandshakeRequest;
use bittorrent_starter_rust::tracker::{TrackerRequest, TrackerResponse, url_encode};


#[tokio::main]
// Usage: yo
async fn main() -> anyhow::Result<()>
{
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
                let t = Torrent::try_from(torrent)?;
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
        Peers { torrent } =>
            {
                let t = Torrent::try_from(torrent)?;
                if let Keys::SingleFile { length } = t.info.keys
                {
                    let tracker_request = TrackerRequest::new(String::from("011112012313"), length);

                    let url_params = serde_urlencoded::to_string(tracker_request).
                        context("URL-tracker params")?;
                    let info_hash = t.info_hash()?;
                    let tracker_url = format!("{}?{}&info_hash={}", t.announce, url_params, &url_encode(&info_hash));


                    let response = reqwest::get(tracker_url).await.context("Fetch Tracker")?;
                    let response: TrackerResponse = serde_json::from_slice(&response.bytes().await
                        .context("Fetch tracker response")?)
                        .context("Serialising response bytes")?;

                    for peer in response.peers.0 {
                        println!("{}:{}", peer.ip(), peer.port());
                    }
                }
            }
        Handshake { torrent, peer } =>
            {
                let t = Torrent::try_from(torrent)?;
                println!("Tracked url: {}", t.announce);

                let hash = t.info_hash()?;
                let socket = SocketAddrV4::from_str(peer).context("Parsing socket")?;
                let handshake = HandshakeRequest::new(hash);
                let mut peer = tokio::net::TcpStream::connect(socket).await.context("Creating connection to peer")?;
                let handshake_bytes = handshake.to_bytes();
                peer.write_all(&handshake_bytes).await.context("Writing to peer")?;

                let mut buffer = Vec::new();
                peer.read_to_end(&mut buffer).await?;
                let response_handshake = HandshakeRequest::from_bytes(&buffer).context("Getting back handshake")?;
                println!("Handshake: {:?}", response_handshake);
            }
    }
    Ok(())
}



