use crate::rpc::Rpc;
use crate::utils::{DeepHashChunk::Data, DeepHashChunk::List};
use crate::wallet::Wallet;
use serde::Serialize;
use crate::{
    common::Tag,
    utils::{deep_hash, hash_256},
};
use anyhow::{Context, Result};
use base64::{prelude::*, engine::general_purpose::URL_SAFE_NO_PAD};

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
    fn max_byte_range(&self) -> u32 {
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
    min_byte_range: u32,
    max_byte_range: u32,
}

#[derive(Clone)]
pub struct BranchNode {
    id: Vec<u8>,
    r#type: String,
    byte_range: u32,
    max_byte_range: u32,
    left_child: Option<Box<MerkelNode>>,
    right_child: Option<Box<MerkelNode>>,
}

#[derive(Default, Serialize, Debug)]
pub struct Chunk {
    data_hash: Vec<u8>,
    min_byte_range: u32,
    max_byte_range: u32,
}

#[derive(Default, Serialize, Debug)]
pub struct Proof {
    offset: u32,
    proof: Vec<u8>,
}

#[derive(Default, Serialize, Debug)]
pub struct Chunks {
    data_root: Vec<u8>,
    chunks: Vec<Chunk>,
    proofs: Vec<Proof>,
}

#[derive(Default, Serialize, Debug)]
pub struct Transaction {
    format: u8,
    id: String,
    last_tx: String,
    owner: String,
    tags: Vec<Tag>,
    target: String,
    quantity: String,
    data: Vec<u8>,
    reward: String,
    chunks: Option<Chunks>,
    signature: String,
    data_size: String,
    data_root: String,
}

impl Transaction {
    pub fn builder(rpc: Rpc) -> TransactionBuilder {
        TransactionBuilder::new(rpc)
    }

    pub fn set_owner(&mut self, owner: String) {
        self.owner = owner
    }

    pub fn prepare_chunks(&mut self, data: Vec<u8>) {
        let length = data.len();
        if self.chunks.is_none() && length > 0 {
            let chunks = generate_transaction_chunks(data);
            self.data_root = URL_SAFE_NO_PAD.encode(&chunks.data_root);
            self.chunks = Some(chunks);
        }
        if self.chunks.is_none() && length == 0 {
            self.chunks = Some(Chunks {
                chunks: Vec::new(),
                data_root: Vec::new(),
                proofs: Vec::new(),
            });
            self.data_root = String::new();
        }
    }
    pub fn get_signature_data(&mut self) -> Result<Vec<u8>> {
        match self.format {
            1 => {
                let owner = BASE64_STANDARD.decode(&self.owner)?;
                let target = BASE64_STANDARD.decode(&self.target)?;
                let quantity = self.quantity.clone().into_bytes();
                let reward = self.reward.clone().into_bytes();
                let last_tx = BASE64_STANDARD.decode(&self.last_tx)?;
                let tags: Vec<u8> = self.tags.iter().fold(Vec::new(), |mut a, tag| {
                    a.extend(BASE64_STANDARD.decode(&tag.name).unwrap());
                    a.extend(BASE64_STANDARD.decode(&tag.value).unwrap());
                    a
                });
                Ok(owner
                    .into_iter()
                    .chain(target)
                    .chain(quantity)
                    .chain(reward)
                    .chain(last_tx)
                    .chain(tags)
                    .collect())
            }
            2 => {
                if self.data_root.is_empty() {
                    self.prepare_chunks(self.data.clone());
                }

                let tags = self.tags.iter().fold(Vec::new(), |mut a, tag| {
                    a.push(List(vec![
                        Data(BASE64_STANDARD.decode(&tag.name).unwrap()),
                        Data(BASE64_STANDARD.decode(&tag.value).unwrap()),
                    ]));
                    a
                });
                //   const tagList: [Uint8Array, Uint8Array][] = this.tags.map((tag) => [
                //     tag.get("name", { decode: true, string: false }),
                //     tag.get("value", { decode: true, string: false }),
                //   ]);

                //   return await deepHash([
                //     ArweaveUtils.stringToBuffer(this.format.toString()),
                //     this.get("owner", { decode: true, string: false }),
                //     this.get("target", { decode: true, string: false }),
                //     ArweaveUtils.stringToBuffer(this.quantity),
                //     ArweaveUtils.stringToBuffer(this.reward),
                //     this.get("last_tx", { decode: true, string: false }),
                //     tagList,
                //     ArweaveUtils.stringToBuffer(this.data_size),
                //     this.get("data_root", { decode: true, string: false }),
                //   ]);
                let format = Data(self.format.to_string().as_bytes().to_vec());
                let owner = Data(URL_SAFE_NO_PAD.decode(&self.owner).unwrap());
                let target = Data(URL_SAFE_NO_PAD.decode(&self.target).unwrap());
                let quantity = Data(self.quantity.clone().into_bytes());
                let reward = Data(self.reward.clone().into_bytes());
                let last_tx = Data(URL_SAFE_NO_PAD.decode(&self.last_tx).unwrap());
                let tags = List(tags);
                let data_size = Data(self.data_size.as_bytes().to_vec());
                let data_root = Data(URL_SAFE_NO_PAD.decode(&self.data_root).unwrap());
                Ok(deep_hash(List(vec![
                    format, owner, target, quantity, reward, last_tx, tags, data_size, data_root,
                ])))
            }
            _ => unreachable!(),
        }
    }

    pub fn sign(
        &mut self,
        jwk: Wallet, 
      ) -> Result<String> {
        self.set_owner(jwk.n.clone());
    
        let data_to_sign = self.get_signature_data()?;
        let raw_signature: Vec<u8> = jwk.sign(&data_to_sign)?;
        let id = hash_256(&raw_signature);
        self.signature = URL_SAFE_NO_PAD.encode(raw_signature);
        self.id = URL_SAFE_NO_PAD.encode(id);
        Ok(self.signature.clone())
      }
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
            min_byte_range: (cursor - chunk.len()) as u32,
            max_byte_range: cursor as u32,
        });
        rest = rest[chunk_size..rest.len()].to_vec();
    }

    chunks.push(Chunk {
        data_hash: hash_256(&rest),
        min_byte_range: cursor as u32,
        max_byte_range: (cursor + rest.len()) as u32,
    });

    return chunks;
}

fn generate_leaves(chunks: &[Chunk]) -> Vec<MerkelNode> {
    chunks
        .into_iter()
        .map(|chunk| {
            return MerkelNode::Leaf(LeafNode {
                r#type: "leaf".to_string(),
                id: hash_256(
                    hash_256(&chunk.data_hash)
                        .into_iter()
                        .chain(hash_256(&chunk.max_byte_range.to_be_bytes()))
                        .collect::<Vec<u8>>()
                        .as_ref(),
                ),
                //   id: await hash(
                // await Promise.all([hash(dataHash), hash(intToBuffer(maxByteRange))])
                //   ),
                data_hash: chunk.data_hash.to_owned(),
                min_byte_range: chunk.min_byte_range,
                max_byte_range: chunk.max_byte_range,
            });
        })
        .collect()
}

fn build_layers(nodes: Vec<MerkelNode>, level: usize) -> MerkelNode {
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
            Some(nodes[i + 1].to_owned()),
        ));
        i += 2
    }

    // console.log("Layer", nextLayer);

    return build_layers(next_layer, level + 1);
}

fn hash_branch(left: MerkelNode, right: Option<MerkelNode>) -> MerkelNode {
    if right.is_none() {
        return left;
    }
    let right = right.unwrap();
    let branch = BranchNode {
        r#type: "branch".to_string(),
        id: hash_256(&left.id())
            .into_iter()
            .chain(hash_256(&right.id()).into_iter())
            .chain(hash_256(&left.max_byte_range().to_be_bytes()))
            .collect::<Vec<u8>>(),
        byte_range: left.max_byte_range(),
        max_byte_range: right.max_byte_range(),
        left_child: Some(Box::new(left)),
        right_child: Some(Box::new(right)),
    };

    MerkelNode::Branch(branch)
}

fn generate_transaction_chunks(data: Vec<u8>) -> Chunks {
    let mut chunks = chunk_data(data);
    let leaves = generate_leaves(&chunks);
    let root = build_layers(leaves, 0);
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
    let proofs = resolve_branch_proofs(root, None, 0);
    proofs
}

fn resolve_branch_proofs(
    node: MerkelNode,
    proof: Option<Vec<u8>>,
    depth: u8
  ) -> Vec<Proof> {
    match &node {
        MerkelNode::Branch(b) => {
            let left =  b.left_child.clone()
                .map(|c| c.id())
                .unwrap_or_default();
            let right =  b.right_child.clone()
                .map(|c| c.id())
                .unwrap_or_default();
            let partial: Vec<u8> = proof.unwrap_or_default()
                .into_iter()
                .chain(left)
                .chain(right).collect();
            return vec![
                resolve_branch_proofs(*b.left_child.clone().unwrap(), Some(partial.clone()), depth + 1),
                resolve_branch_proofs(*b.right_child.clone().unwrap(), Some(partial), depth + 1),
            ].into_iter().flatten().collect();
        },
        MerkelNode::Leaf(l) => {
            return vec![Proof {
                offset: node.max_byte_range() - 1,
                proof: proof.unwrap_or_default().into_iter().chain(l.data_hash.clone()).chain(node.max_byte_range().to_be_bytes()).collect()
            }]

        }
        _ => unreachable!()
    }
  }

#[derive(Default)]
pub struct TransactionBuilder {
    rpc: Rpc,
    format: Option<u8>,
    last_tx: Option<String>,
    owner: Option<String>,
    tags: Option<Vec<Tag>>,
    target: Option<String>,
    quantity: Option<String>,
    data: Option<Vec<u8>>,
    data_size: Option<String>,
    data_root: Option<String>,
    reward: Option<String>
}

impl TransactionBuilder {
    pub fn new(rpc: Rpc) -> Self {
        Self {
            rpc,
            ..Default::default()
        }
    }

    pub fn format(mut self, format: u8) -> Self {
        self.format = Some(format);
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn quantity(mut self, quantity: String) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    pub fn owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }

    pub async fn build(self) -> Result<Transaction> {
        let last_tx = if let Some(tx) = self.last_tx {
            tx
        } else {
            self.rpc.anchor().await?
        };
        let data = self.data.context("No data supplied")?;
        let reward = if let Some(reward) = self.reward {
            reward
        } else {
            self.rpc.get_price(data.len(), &self.target).await?
        };
        let mut tx = Transaction { 
            format: self.format.unwrap_or(2), 
            id: String::new(), 
            last_tx, 
            owner: self.owner.unwrap_or_default(),
            tags: self.tags.unwrap_or_default(), 
            target: self.target.unwrap_or_default(), 
            quantity: self.quantity.unwrap_or("0".to_string()), 
            data_size: data.len().to_string(),
            data, 
            reward,
            ..Default::default()
        };
        tx.get_signature_data().unwrap();
        Ok(tx)
    }
}