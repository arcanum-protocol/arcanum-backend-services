use crate::utils::{hash_256, pad_to_32_bytes};
use serde::Serialize;

const MAX_CHUNK_SIZE: usize = 256 * 1024;
const MIN_CHUNK_SIZE: usize = 32 * 1024;

#[derive(Clone)]
pub enum MerkelNode {
    Leaf(LeafNode),
    Branch(BranchNode),
}

impl MerkelNode {
    fn id(&self) -> Vec<u8> {
        match self {
            MerkelNode::Branch(v) => v.id.clone(),
            MerkelNode::Leaf(v) => v.id.clone(),
        }
    }
    fn max_byte_range(&self) -> usize {
        match self {
            MerkelNode::Branch(v) => v.max_byte_range,
            MerkelNode::Leaf(v) => v.max_byte_range,
        }
    }
}

#[derive(Clone)]
pub struct LeafNode {
    id: Vec<u8>,
    data_hash: Vec<u8>,
    r#type: String,
    min_byte_range: usize,
    max_byte_range: usize,
}

#[derive(Clone)]
pub struct BranchNode {
    id: Vec<u8>,
    r#type: String,
    byte_range: usize,
    max_byte_range: usize,
    left_child: Option<Box<MerkelNode>>,
    right_child: Option<Box<MerkelNode>>,
}

#[derive(Default, Serialize, Debug)]
pub struct Chunk {
    pub data_hash: Vec<u8>,
    pub min_byte_range: usize,
    pub max_byte_range: usize,
}

#[derive(Default, Serialize, Debug)]
pub struct Proof {
    pub offset: usize,
    pub proof: Vec<u8>,
}

#[derive(Default, Serialize, Debug)]
pub struct Chunks {
    pub data_root: Vec<u8>,
    pub chunks: Vec<Chunk>,
    pub proofs: Vec<Proof>,
}

fn chunk_data(data: Vec<u8>) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();

    let mut rest = data;
    let mut cursor = 0;

    while rest.len() >= MAX_CHUNK_SIZE {
        let mut chunk_size = MAX_CHUNK_SIZE;

        // If the total bytes left will produce a chunk < MIN_CHUNK_SIZE,
        // then adjust the amount we put in this 2nd last chunk.

        let next_chunk_size = rest.len() - MAX_CHUNK_SIZE;
        if next_chunk_size > 0 && next_chunk_size < MIN_CHUNK_SIZE {
            chunk_size = (rest.len() / 2) + (rest.len() % 2 == 0) as usize;
            // console.log(`Last chunk will be: ${nextChunkSize} which is below ${MIN_CHUNK_SIZE}, adjusting current to ${chunkSize} with ${rest.byteLength} left.`)
        }

        let chunk = &rest[0..chunk_size];
        let data_hash = hash_256(chunk);
        cursor += chunk.len();
        chunks.push(Chunk {
            data_hash,
            min_byte_range: cursor - chunk.len(),
            max_byte_range: cursor,
        });
        rest = rest[chunk_size..rest.len()].to_vec();
    }

    chunks.push(Chunk {
        data_hash: hash_256(&rest),
        min_byte_range: cursor,
        max_byte_range: cursor + rest.len(),
    });

    chunks
}

fn generate_leaves(chunks: &[Chunk]) -> Vec<MerkelNode> {
    chunks
        .iter()
        .map(|chunk| {
            MerkelNode::Leaf(LeafNode {
                r#type: "leaf".to_string(),
                id: hash_256(
                    hash_256(&chunk.data_hash)
                        .into_iter()
                        .chain(hash_256(&pad_to_32_bytes(
                            &chunk.max_byte_range.to_be_bytes(),
                        )))
                        .collect::<Vec<u8>>()
                        .as_ref(),
                ),
                //   id: await hash(
                // await Promise.all([hash(dataHash), hash(intToBuffer(maxByteRange))])
                //   ),
                data_hash: chunk.data_hash.to_owned(),
                min_byte_range: chunk.min_byte_range,
                max_byte_range: chunk.max_byte_range,
            })
        })
        .collect()
}

fn build_layers(nodes: Vec<MerkelNode>) -> MerkelNode {
    // If there is only 1 node left, this is going to be the root node
    if nodes.len() < 2 {
        let root = nodes[0].to_owned();

        // console.log("Root layer", root);

        return root;
    }

    let mut next_layer: Vec<MerkelNode> = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        next_layer.push(hash_branch(
            nodes[i].to_owned(),
            nodes.get(i + 1).map(|v| v.clone()),
        ));
        i += 2
    }

    // console.log("Layer", nextLayer);

    build_layers(next_layer)
}

fn hash_branch(left: MerkelNode, right: Option<MerkelNode>) -> MerkelNode {
    if right.is_none() {
        return left;
    }
    let right = right.unwrap();
    let branch = BranchNode {
        r#type: "branch".to_string(),
        id: hash_256(
            &hash_256(&left.id())
                .into_iter()
                .chain(hash_256(&right.id()))
                .chain(hash_256(&pad_to_32_bytes(
                    &left.max_byte_range().to_be_bytes(),
                )))
                .collect::<Vec<u8>>(),
        ),
        byte_range: left.max_byte_range(),
        max_byte_range: right.max_byte_range(),
        left_child: Some(Box::new(left)),
        right_child: Some(Box::new(right)),
    };

    MerkelNode::Branch(branch)
}

pub fn generate_transaction_chunks(data: Vec<u8>) -> Chunks {
    let mut chunks = chunk_data(data);
    let leaves = generate_leaves(&chunks);
    let root = build_layers(leaves);
    // todo add proofs
    let id = root.id();
    let mut proofs = generate_proofs(root);
    // Discard the last chunk & proof if it's zero length.
    let last_chunk = chunks.last();
    if last_chunk.is_some()
        && last_chunk.unwrap().max_byte_range - last_chunk.unwrap().min_byte_range == 0
    {
        chunks.pop();
        proofs.pop();
    }

    Chunks {
        data_root: id,
        chunks,
        proofs,
    }
}

fn generate_proofs(root: MerkelNode) -> Vec<Proof> {
    resolve_branch_proofs(root, None)
}

fn resolve_branch_proofs(node: MerkelNode, proof: Option<Vec<u8>>) -> Vec<Proof> {
    match &node {
        MerkelNode::Branch(b) => {
            let left = b.left_child.clone().map(|c| c.id()).unwrap_or_default();
            let right = b.right_child.clone().map(|c| c.id()).unwrap_or_default();
            let partial: Vec<u8> = proof
                .unwrap_or_default()
                .into_iter()
                .chain(left)
                .chain(right)
                .collect();
            vec![
                resolve_branch_proofs(*b.left_child.clone().unwrap(), Some(partial.clone())),
                resolve_branch_proofs(*b.right_child.clone().unwrap(), Some(partial)),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        MerkelNode::Leaf(l) => {
            vec![Proof {
                offset: node.max_byte_range() - 1,
                proof: proof
                    .unwrap_or_default()
                    .into_iter()
                    .chain(l.data_hash.iter().copied())
                    .chain(node.max_byte_range().to_be_bytes())
                    .collect(),
            }]
        }
    }
}
