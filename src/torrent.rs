use std::path::{Path, PathBuf};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use crate::downloaded;
use crate::downloaded::Downloaded;
use crate::hashes::Hashes;


#[derive(Deserialize, Debug, Clone)]
pub struct Torrent
{
    pub announce: String,
    pub info: Info,
}

impl Torrent
{
    pub fn len(&self) -> usize
    {
        match &self.info.keys {
            Keys::SingleFile {length} => *length,

            Keys::MultiFile {files} => {let mut sum  = 0; files.iter().for_each(|file|sum += file.length); sum}
        }
    }
    pub fn info_hash(&self) -> anyhow::Result<[u8; 20]>
    {
        let re_encoded = serde_bencode::to_bytes(&self.info)?;

        let mut hash = Sha1::new();
        hash.update(re_encoded);
        Ok(hash.finalize().into())
    }
    pub fn read(file: impl AsRef<Path>) -> anyhow::Result<Self>
    {
        let f = std::fs::read(file).context("Read torrent file")?;
        Self::try_from(f)
    }
    pub fn print_tree(&self)
    {
        fn print_subtree() {}

        match &self.info.keys
        {
            Keys::SingleFile { .. } =>
                {
                    println!("{}", self.info.name)
                },
            Keys::MultiFile { files } => {
                for file in files
                {
                    println!("{:?}", file.path.join(std::path::MAIN_SEPARATOR_STR))
                }
            }
        }
    }
    pub async fn download_all(&self, peer_id: String) -> anyhow::Result<Downloaded>
    {
        downloaded::all(self, peer_id).await
    }
}


impl TryFrom<&PathBuf> for Torrent
{
    type Error = anyhow::Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let f = std::fs::read(value).context("Read torrent file")?;
        serde_bencode::from_bytes(&f).context("Parse torrent file")
    }
}

impl TryFrom<Vec<u8>> for Torrent
{
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_bencode::from_bytes(&value).context("Parse torrent file")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info
{
    pub name: String,
    pub length: usize,
    #[serde(rename(deserialize = "piece length"))]
    pub piece_length: usize,
    pub pieces: Hashes,
    #[serde(flatten)]
    pub keys: Keys,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Keys
{
    SingleFile
    {
        length: usize
    },
    MultiFile
    {
        files: Vec<File>
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct File
{
    pub length: usize,
    pub path: Vec<String>,
}