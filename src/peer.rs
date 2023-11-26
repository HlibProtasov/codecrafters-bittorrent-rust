use anyhow::Context;


#[derive(Debug)]
pub struct Handshake
{
    length: u8,
    bit_torrent: [u8;19],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8;20],
}

impl Handshake
{
    const SIZE: usize = 68;

    pub fn new(info_hash: [u8; 20]) -> Self
    {
        Self
        {
            length: 19,
            bit_torrent: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id: *b"00112233445566778890",
        }
    }
    pub fn to_bytes(&self) -> Vec<u8>
    {

        let mut bytes = Vec::with_capacity(Self::SIZE);

        bytes.push(self.length);
        bytes.extend(&self.bit_torrent);
        bytes.extend(&self.reserved);
        bytes.extend(&self.info_hash);
        bytes.extend(&self.peer_id);

        bytes

    }
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>
    {
        if bytes.len() != Self::SIZE
        {
            return Err(anyhow::Error::msg("Should be 68 length bytes array"));
        }
        let handshake = Self
        {
            length: bytes[0],
            bit_torrent: bytes[1..20].try_into().context("Creating bit torrent array")?,
            reserved: bytes[20..28].try_into().context("Creating reserved array")?,
            info_hash: bytes[28..48].try_into().context("Creating info hash array")?,
            peer_id: bytes[48..68].try_into().context("Creating peer id array")?,
        };
        Ok(handshake)
    }
}

#[cfg(test)]
mod test_handhaske_conversion
{
    use crate::peer::Handshake;

    #[test]
    fn to_bytes()
    {
        let info_hash = [1_u8; 20];
        let handshake = Handshake::new(info_hash);

        let handshake_bytes = handshake.to_bytes();


        assert_eq!(handshake_bytes.len(), Handshake::SIZE, "Wrong length");
    }

    #[test]
    fn from_bytes()
    {
        let info_hash = [1_u8; 20];
        let handshake = Handshake::new(info_hash);

        let handshake_bytes = handshake.to_bytes();

        let handshake = Handshake::from_bytes(&handshake_bytes);

        assert!(handshake.is_ok());

        let handshake = handshake.unwrap();
        assert_eq!(handshake.info_hash, info_hash, "Wrong info hash");
        assert_eq!(handshake.length, 19, "Wrong len");
        assert_eq!(handshake.bit_torrent, *b"BitTorrent protocol", "Wrong bitorrent");
        assert_eq!(handshake.peer_id, *b"00112233445566778890", "Wrong peer id");
        assert_eq!(handshake.reserved, [0; 8], "Wrong reserved");
    }
}
