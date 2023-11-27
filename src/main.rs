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
use bittorrent_starter_rust::TorrentExecutor::TorrentExecutor;
use bittorrent_starter_rust::tracker::{TrackerRequest, TrackerResponse, url_encode};


#[tokio::main]
// Usage: yo
async fn main() -> anyhow::Result<()>
{
    let cli = Cli::parse();

    TorrentExecutor::execute(cli.command).await?;

    Ok(())
}



