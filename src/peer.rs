use anyhow::Context;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use bytes::{BytesMut, Buf};

#[derive(Debug)]
pub struct Handshake
{
    length: u8,
    bit_torrent: [u8; 19],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
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


#[derive(Debug)]
#[repr(u8)]
pub enum MessageTag
{
    Choke = 0,
    UnChoke = 1,
    Interested = 2,
    NonInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

#[derive(Debug)]
pub struct Message
{
    pub tag: MessageTag,
    pub payload: Vec<u8>,
}


pub struct MessageFramer;

const MAX: usize = 2 ^ 16;

impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 5 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }


        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        if src[5] > 8

        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid message Tag. Expected a value in the range [0, 8], found: {}", src[5]),
            ));
        }
        let message_tag: MessageTag = unsafe { std::mem::transmute(src[5]) };
        let data = src[5..4 + length - 1].to_vec();
        src.advance(4 + length);

        Ok(
            Some(
                Message
                {
                    tag: message_tag,
                    payload: data,
                }
            )
        )
    }
}
impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send a message if it is longer than the other end will
        // accept.
        if item.payload.len() + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", item.payload.len() + 1)
            ));
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_be_bytes(item.payload.len() as u32 + 1);

        // Reserve space in the buffer.
        dst.reserve(4 + 1 + item.payload.len());

        let tag: u8 = unsafe { std::mem::transmute(item.tag) };
        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(&[tag]);
        dst.extend_from_slice(item.payload.as_slice());
        Ok(())
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