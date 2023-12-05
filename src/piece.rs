use std::cmp::Ordering;
use crate::peer::Peer;
use crate::torrent::Torrent;
use crate::tracker::TrackerResponse;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Piece
{
    peers: Vec<usize>,
    piece_i: u64,
    length: usize,
    info_hash: [u8; 20],
    seed: u8
}
impl Piece
{
    pub fn new(piece_i: u64, torrent: &Torrent, peers: &[Peer]) -> Self
    {
        let piece_size = match piece_i == torrent.info.pieces.0.len() {
            true if torrent.len() % torrent.info.piece_length == 0 =>
                torrent.info.piece_length,
            true => torrent.len() % torrent.info.piece_length,
            _ => torrent.info.piece_length
        };
        let peers = peers.iter().enumerate().filter_map(
            |(peer_i, peer)|peer.has_piece(piece_i as u32).then_some(piece_i)).collect();

        Self
        {
            peers,
            piece_i,
            length: piece_size,
            info_hash: torrent.info.pieces.0[piece_i],
            seed: fastrand::u8(..),
        }
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