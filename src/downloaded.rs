use std::collections::BinaryHeap;
use std::net::SocketAddrV4;
use std::slice::Iter;
use anyhow::Context;
use futures_util::stream::StreamExt;
use crate::peer::Peer;
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

    let mut peer_list = Vec::new();

    /// TODO: in parallel
   let mut stream = futures_util::stream::iter(tracker_response.peers.0.iter()).map(
       |peer|

    Peer::new(*peer, torrent.info_hash()?)
   ).buffer_unordered(5/*TODO user config**/);
    while let Some(peer) = stream.next().await {
        match peer {
            Some(peer) => peer_list.push(peer),

            Err(e) =>
                eprintln!("Fail to connect ot peer: {:?} with error: {}", peer, e)
        }

    }
    drop(stream);



          todo!()
      }
       pub(crate) async fn download_piece(peers: &[SocketAddrV4], piece_hash:[u8;20],piece_len: usize) -> anyhow::Result<Downloaded>
       {
           todo!()

   }
    pub(crate) async fn download_piece_block(peer: &SocketAddrV4, piece_hash:[u8;20],piece_len: usize) -> anyhow::Result<Downloaded>
    {

    todo!()
    }






impl<'a> IntoIterator for &'a Downloaded
{
    type Item = DownloadedFile<'a>;
    type IntoIter = DownloadedIter <'a>;

    fn into_iter(self) -> Self::IntoIter {
        DownloadedIter::new(self)
    }
}
pub struct DownloadedIter<'a>
{
    downloaded: &'a Downloaded,
    files: Iter<'a, File>,
    offset: usize

}
impl<'a> DownloadedIter <'a>
{
    pub fn new(downloaded: &'a Downloaded) -> Self
    {
        Self
        {
            downloaded,
            files: downloaded.file.iter(),
            offset: 0
        }

    }
}
impl<'a> Iterator for DownloadedIter<'a>
{
    type Item = DownloadedFile<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.files.next()?;
        let bytes = &self.downloaded.bytes[self.offset..][.. file.length];
        Some(
            DownloadedFile
        {
            file,
            bytes
        }
        )
    }
}
pub struct DownloadedFile<'a>
{
    file: &'a File,
    bytes: &'a [u8]
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