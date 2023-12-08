use std::cmp::Ordering;
use std::collections::HashSet;
use crate::peer::Peer;
use crate::torrent::Torrent;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Piece
{
    peers: HashSet<usize>,
    piece_i: u64,
    length: usize,
    info_hash: [u8; 20],
    seed: u8,
}

impl Piece
{
    pub fn new(piece_i: u64, torrent: &Torrent, peers: &[Peer]) -> Self
    {
        let piece_size = match piece_i as usize == torrent.info.pieces.0.len() {
            true if torrent.len() % torrent.info.piece_length == 0 =>
                torrent.info.piece_length,
            true => torrent.len() % torrent.info.piece_length,
            _ => torrent.info.piece_length
        };
        let peers = peers.iter().enumerate().filter_map(
            |(peer_i, peer)| peer.has_piece(piece_i as u32).then_some(piece_i as usize)).collect();

        Self
        {
            peers,
            piece_i,
            length: piece_size,
            info_hash: torrent.info.pieces.0[piece_i as usize],
            seed: fastrand::u8(..),
        }
    }
    pub(crate) fn peers(&self) -> &HashSet<usize>
    {
        &self.peers
    }
    pub fn length(&self) -> usize
    {
        self.length
    }
    pub fn index(&self) -> u64
    {
        self.piece_i
    }
    pub fn hash(&self) -> [u8;20]
    {
        self.info_hash
    }
}

impl Ord for Piece
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.peers
            .len()
            .cmp(&other.peers.len())
            .then(self.seed.cmp(&other.seed))
            .then(self.piece_i.cmp(&other.piece_i))
            .then(self.length.cmp(&other.length))
            .then(self.info_hash.cmp(&other.info_hash))
    }
}

impl PartialOrd for Piece
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}