use rs_sha384::{HasherContext, Sha384State};
use sha256::digest;
use std::hash::{BuildHasher, Hasher};

pub fn hash_384(data: &[u8]) -> Vec<u8> {
    let mut sha384hasher = Sha384State::default().build_hasher();
    sha384hasher.write(data);
    HasherContext::finish(&mut sha384hasher).as_ref().to_vec()
}

pub fn hash_256(data: &[u8]) -> Vec<u8> {
    digest(data).into_bytes()
}

pub fn concat_buffers(buffs: Vec<Vec<u8>>) -> Vec<u8> {
    let mut res = Vec::new();
    for buf in buffs.into_iter() {
        res.extend(buf);
    }
    return res;
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
                .to_vec()
                .into_iter()
                .chain(list.len().to_string().as_bytes().to_vec())
                .collect::<Vec<u8>>();
            return deep_hash_chunks(list, hash_384(&tag));
        }
        DeepHashChunk::Data(data) => {
            let tag = "blob"
                .as_bytes()
                .to_vec()
                .into_iter()
                .chain(data.len().to_string().as_bytes().to_vec().into_iter())
                .collect::<Vec<u8>>();
            let tagged_hash = hash_384(&tag)
                .into_iter()
                .chain(hash_384(&data).into_iter())
                .collect::<Vec<u8>>();
            return hash_384(&tagged_hash);
        }
    }
}

pub fn deep_hash_chunks(chunks: Vec<DeepHashChunk>, acc: Vec<u8>) -> Vec<u8> {
    if chunks.len() < 1 {
        return acc;
    }

    let hash_pair = acc
        .into_iter()
        .chain(deep_hash(chunks[0].clone()))
        .collect::<Vec<u8>>();

    let new_acc = hash_384(&hash_pair);
    let mut chunks = chunks;
    chunks.remove(0);
    return deep_hash_chunks(chunks, new_acc);
}


// bufferToString
// String::from_utf8()

// stringToBuffer
// .as_bytes()
