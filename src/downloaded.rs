use std::collections::BinaryHeap;
use std::net::SocketAddrV4;
use std::slice::Iter;
use anyhow::Context;
use futures_util::stream::StreamExt;
use crate::peer::{Message, MessageTag, Peer, PeerRequest, PieceMessage};
use crate::peer::MessageTag::Request;
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

    let peer_list = {
        let mut peer_list = Vec::new();

        let mut stream = futures_util::stream::iter(tracker_response.peers.0.iter()).map(
            |peer|

                Peer::new(*peer, torrent.info_hash()?)
        ).buffer_unordered(5/*TODO user config**/);
        while let Some(peer) = stream.next().await {
            match peer {
                Ok(peer) => peer_list.push(peer),

                Err(e) =>
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
        let nblocks = (piece.length() + Peer::BLOCK_MAX as usize - 1) / Peer::BLOCK_MAX as usize;
        let mut pieces: Vec<u8> = Vec::with_capacity(nblocks);
        for block in 0..nblocks
        {
            let md = piece.length() % Peer::BLOCK_MAX as usize;
            let block_size = match block == nblocks {
                true if md != 0 => md as u32,
                _ => Peer::BLOCK_MAX
            };
            let request = PeerRequest::new(piece.index() as u32, (block as u32 * Peer::BLOCK_MAX), block_size);
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
            assert!(!msg.payload.is_empty(), "Shouldn't be empty");
            let piece = PieceMessage::from_bytes(msg.payload.as_slice());
            pieces.extend(piece.block());
        }
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