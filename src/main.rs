use anyhow::Context;
use ::bittorrent_starter_rust::{
    decoder::decode_bencoded_value,
    cli::{
        Commands::{
            Decode,
            Info as CInfo,
            Peers,
        },
        Cli,
    },
    Torrent,
    Keys,
};
use clap::Parser;

use sha1::{Sha1, Digest};
use bittorrent_starter_rust::tracker::TrackerRequest;


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
                let hash = t.info_hash()?;
                if let Keys::SingleFile { length } = t.info.keys {
                    let tracker_request = TrackerRequest::new(hash.into(), String::from("011112012313"), length);
                    let mut url = reqwest::Url::parse(&t.announce)?;
                    let url_params = serde_urlencoded::to_string(tracker_request).
                        context("URL-tracker params")?;
                    url.set_query(Some(&url_params));
                    let response = reqwest::get(url).await?;
                }
            }
    }
    Ok(())
}



