use crate::chunks::{generate_transaction_chunks, Chunks};
use crate::rpc::Rpc;
use crate::tx_builder::TransactionBuilder;
use crate::utils::{deep_hash, hash_256};
use crate::utils::{DeepHashChunk::Data, DeepHashChunk::List};
use crate::wallet::Signer;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, prelude::*};
use serde::{Serialize, Serializer};

#[derive(Default, Debug, Serialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
pub struct GetChunk {
    pub data_root: String,
    pub data_size: String,
    pub data_path: String,
    pub offset: String,
    pub chunk: String,
}

#[derive(Default, Serialize, Debug)]
pub struct Transaction {
    pub format: u8,
    pub id: String,
    pub last_tx: String,
    pub owner: String,
    pub tags: Vec<Tag>,
    pub target: String,
    pub quantity: String,
    #[serde(serialize_with = "data_serializer")]
    pub data: Vec<u8>,
    pub reward: String,
    #[serde(skip_serializing)]
    pub chunks: Option<Chunks>,
    pub signature: String,
    pub data_size: String,
    pub data_root: String,
}

fn data_serializer<S>(data: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let base = URL_SAFE_NO_PAD.encode(data);
    s.serialize_str(&base)
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
                let owner = URL_SAFE_NO_PAD.decode(&self.owner)?;
                let target = URL_SAFE_NO_PAD.decode(&self.target)?;
                let quantity = self.quantity.clone().into_bytes();
                let reward = self.reward.clone().into_bytes();
                let last_tx = URL_SAFE_NO_PAD.decode(&self.last_tx)?;
                let tags: Vec<u8> = self.tags.iter().fold(Vec::new(), |mut a, tag| {
                    a.extend(URL_SAFE_NO_PAD.decode(&tag.name).unwrap());
                    a.extend(URL_SAFE_NO_PAD.decode(&tag.value).unwrap());
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
                        Data(URL_SAFE_NO_PAD.decode(&tag.name).unwrap()),
                        Data(URL_SAFE_NO_PAD.decode(&tag.value).unwrap()),
                    ]));
                    a
                });
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

    pub fn get_chunk(&self, idx: usize, data: &[u8]) -> Result<GetChunk> {
        if let Some(chunks) = &self.chunks {
            if chunks.proofs.len() > idx && chunks.chunks.len() > idx {

            let proof = &chunks.proofs[idx];
            let chunk = &chunks.chunks[idx];
            Ok(GetChunk {
                data_root: self.data_root.clone(),
                data_size: self.data_size.clone(),
                data_path: URL_SAFE_NO_PAD.encode(&proof.proof),
                offset: proof.offset.to_string(),
                chunk: URL_SAFE_NO_PAD.encode(&data[chunk.min_byte_range..chunk.max_byte_range]),
            })
            } else {
                Err(anyhow!("No chunk for given index"))
            }
        } else {
            Err(anyhow!("Chunks have not been prepared"))
        }
    }

    pub fn sign(&mut self, jwk: Signer) -> Result<()> {
        self.set_owner(jwk.address.clone());

        let data_to_sign = self.get_signature_data()?;
        let raw_signature: Vec<u8> = jwk.sign(&data_to_sign)?;
        let id = hash_256(&raw_signature);
        self.signature = URL_SAFE_NO_PAD.encode(raw_signature);
        self.id = URL_SAFE_NO_PAD.encode(id);
        Ok(())
    }
}
