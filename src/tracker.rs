use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest
{
    pub info_hash: [u8; 20],
    pub peer_id: String,
    pub port: u16,
    // 6881
    pub uploaded: usize,
    // 0
    pub downloaded: usize,
    // 0
    pub left: usize,
    // the length of the file
    pub compact: bool,
}

impl TrackerRequest
{
    pub fn new(info_hash: [u8; 20], peer_id: String, left: usize) -> Self
    {
        Self
        {
            info_hash,
            peer_id,
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left,
            compact: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TrackerResponse
{
    // in seconds
    interval: usize,
    peers: String,

}

pub mod peers
{
    use std::fmt;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use bytes::Buf;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Clone)]
    pub struct Peers(pub Vec<SocketAddrV4>);

    struct PeersVisitor;

    impl<'de> Visitor<'de> for PeersVisitor {
        type Value = Peers;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a String that first 4 bytes is ip, and last 2 is port number")
        }
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: Error {
            if v.len() % 6 != 0
            {
                return Err(E::custom(format!("Length is {}", v.len())));
            }
            let peers = v.chunks_exact(6)
                .map(|bytes|
                    {
                        SocketAddrV4::new(
                            Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]),
                            u16::from_be_bytes([bytes[4],bytes[5]]),
                        )
                    }).collect();

            Ok(
                Peers(peers)
            )
        }
    }

    impl<'de> Deserialize<'de> for Peers {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>
        {
            deserializer.deserialize_string(PeersVisitor)
        }
    }

    impl Serialize for Peers
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer
        {
            let mut single_slice = Vec::with_capacity(6 * self.0.len());
            for peer in &self.0
            {
               single_slice.extend(peer.ip().octets());
                single_slice.extend(peer.port().to_be_bytes());
            }
            serializer.serialize_bytes(&single_slice)
        }
    }
}

