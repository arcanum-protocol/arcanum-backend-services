use rs_sha384::{HasherContext, Sha384State};
use sha256::digest;
use std::hash::{BuildHasher, Hasher};

pub fn hash_384(data: &[u8]) -> Vec<u8> {
    let mut sha384hasher = Sha384State::default().build_hasher();
    sha384hasher.write(data);
    HasherContext::finish(&mut sha384hasher).as_ref().to_vec()
}

pub fn hash_256(data: &[u8]) -> Vec<u8> {
    hex::decode(digest(data)).unwrap()
}

#[derive(Clone)]
pub enum DeepHashChunk {
    Data(Vec<u8>),
    List(Vec<DeepHashChunk>),
}

pub fn deep_hash(data: DeepHashChunk) -> Vec<u8> {
    match data {
        DeepHashChunk::List(list) => {
            let tag = "list"
                .as_bytes()
                .iter()
                .copied()
                .chain(list.len().to_string().as_bytes().to_vec())
                .collect::<Vec<u8>>();
            deep_hash_chunks(list, hash_384(&tag))
        }
        DeepHashChunk::Data(data) => {
            let tag = "blob"
                .as_bytes()
                .iter()
                .copied()
                .chain(data.len().to_string().as_bytes().iter().copied())
                .collect::<Vec<u8>>();
            let tagged_hash = hash_384(&tag)
                .into_iter()
                .chain(hash_384(&data))
                .collect::<Vec<u8>>();
            hash_384(&tagged_hash)
        }
    }
}

pub fn deep_hash_chunks(chunks: Vec<DeepHashChunk>, acc: Vec<u8>) -> Vec<u8> {
    if chunks.is_empty() {
        return acc;
    }

    let hash_pair = acc
        .into_iter()
        .chain(deep_hash(chunks[0].clone()))
        .collect::<Vec<u8>>();

    let new_acc = hash_384(&hash_pair);
    let mut chunks = chunks;
    chunks.remove(0);
    deep_hash_chunks(chunks, new_acc)
}


pub fn pad_to_32_bytes(original: &[u8]) -> Vec<u8> {
    let mut padded = vec![0u8; 32];
    let len = original.len();
    if len >= 32 {
        padded.copy_from_slice(&original[len - 32..]);
    } else {
        padded[32 - len..].copy_from_slice(original);
    }
    padded
}
// bufferToString
// String::from_utf8()

// stringToBuffer
// .as_bytes()
