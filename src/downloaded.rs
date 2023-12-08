use std::collections::BinaryHeap;
use std::net::SocketAddrV4;
use std::slice::Iter;
use anyhow::Context;
use futures_util::stream::StreamExt;
use sha1::{Sha1, Digest};
use tokio::task::JoinSet;
use crate::peer::{Peer, PieceMessage};
use crate::piece::Piece;
use crate::torrent::{File, Torrent};
use crate::tracker::TrackerResponse;

pub struct Downloaded
{
    bytes: Vec<u8>,
    file: Vec<File>,

}

pub(crate) async fn all(torrent: &Torrent, peer_id: String) -> anyhow::Result<Downloaded>
{
    let tracker_response = TrackerResponse::query(torrent, peer_id).await
        .context("Query tracker for peer info")?;

    let mut peer_list = {
        let mut peer_list = Vec::new();

        let info_hash = torrent.info_hash()?;
        let mut stream = futures_util::stream::iter(tracker_response.peers.0.iter()).map(
            |peer|

                Peer::new(*peer, info_hash)
        ).buffer_unordered(5/*TODO user config**/);
        while let Some(peer) = stream.next().await {
            match peer {
                Ok(peer) => peer_list.push(peer),

                Err(ref e) =>
                    eprintln!("Fail to connect ot peer: {:?} with error: {}", peer, e)
            }
        }
        peer_list
    };
    let mut pieces = BinaryHeap::new();
    let mut no_piece = Vec::new();
    for piece_id in 0..torrent.info.pieces.0.len()
    {
        let piece = Piece::new(piece_id as u64, torrent, &peer_list);

        if piece.peers().is_empty()
        {
            no_piece.push(piece)
        } else {
            pieces.push(piece)
        }
    }
    /// TODO!
    assert!(no_piece.is_empty());

    while let Some(piece) = pieces.pop() {
        let nblocks = ((piece.length() + Peer::BLOCK_MAX as usize - 1) / Peer::BLOCK_MAX as usize) as u32;
        let mut pieces: Vec<u8> = Vec::with_capacity(nblocks as usize);

        let peers_with_piece: Vec<_> = peer_list
            .iter_mut()
            .enumerate()
            .filter_map(|(i, peer)| piece.peers().contains(&i).then_some(peer))
            .collect();

        let (submit, tasks  ) = kanal::bounded_async::<u32>(nblocks as usize);
        for block in 0..nblocks
        {
            submit.send(block).await?;
        }
        let (finish, mut done) = tokio::sync::mpsc::channel(nblocks as usize);
        let mut participants =  futures_util::stream::futures_unordered::FuturesUnordered::new();
        let piece_size = piece.length();
        for peer in peers_with_piece
        {
            participants.push(peer.participate(
                piece.index() as u32,
                piece_size as u32,
                nblocks as u32,
                submit.clone(),
                tasks.clone(),
                finish.clone())
            );
        }

        let mut bytes_recv = 0;
        let mut all_blocks = vec![0u8; piece.length()];
        loop {
            tokio::select! {
                joined = participants.next() =>
                {
                    match joined {
                    None => { }, // the are no peers
                    Some(Ok(_)) => {} // the peer gave up
                    Some(Err(_)) => {} //The peer failed
                }
                    },
                piece = done.recv() =>
                {
                    if let Some(piece) = &piece {
                    let piece = PieceMessage::from_bytes(&piece.payload);
                    bytes_recv += piece.block().len();
                   all_blocks[piece.begin() as usize..].copy_from_slice(piece.block())
                    }
                else {

                    anyhow::ensure!(piece_size == bytes_recv, "Not enough bytes");
                    // have recieved every piece or no peers left
                    break;
                }
                }
            }
        }

        drop(done);
        drop(finish);
        drop(submit);

        let mut sha = Sha1::new();
        sha.update(all_blocks);
        let hash:[u8;20] = sha.finalize().try_into().context("Trying to get hash from blocks")?;
        anyhow::ensure!(hash == piece.hash(),"Got wrong hash from blocks");


    }
    todo!()
}

pub(crate) async fn download_piece(peers: &[SocketAddrV4], piece_hash: [u8; 20], piece_len: usize) -> anyhow::Result<Downloaded>
{
    todo!()
}

pub(crate) async fn download_piece_block(peer: &SocketAddrV4, piece_hash: [u8; 20], piece_len: usize) -> anyhow::Result<Downloaded>
{
    todo!()
}


impl<'a> IntoIterator for &'a Downloaded
{
    type Item = DownloadedFile<'a>;
    type IntoIter = DownloadedIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DownloadedIter::new(self)
    }
}

pub struct DownloadedIter<'a>
{
    downloaded: &'a Downloaded,
    files: Iter<'a, File>,
    offset: usize,
}

impl<'a> DownloadedIter<'a>
{
    pub fn new(downloaded: &'a Downloaded) -> Self
    {
        Self
        {
            downloaded,
            files: downloaded.file.iter(),
            offset: 0,
        }
    }
}

impl<'a> Iterator for DownloadedIter<'a>
{
    type Item = DownloadedFile<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.files.next()?;
        let bytes = &self.downloaded.bytes[self.offset..][..file.length];
        Some(
            DownloadedFile
            {
                file,
                bytes,
            }
        )
    }
}

pub struct DownloadedFile<'a>
{
    file: &'a File,
    bytes: &'a [u8],
}

impl<'a> DownloadedFile<'a>
{
    pub fn path(&self) -> &Vec<String>
    {
        &self.file.path
    }
    pub fn bytes(&self) -> &[u8]
    {
        self.bytes
    }
}