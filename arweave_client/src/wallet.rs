use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rsa::{
    pss::BlindedSigningKey,
    rand_core::OsRng,
    sha2::Sha256,
    signature::{RandomizedSigner, SignatureEncoding},
    BigUint, RsaPrivateKey,
};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Wallet {
    pub n: String,
    pub e: String,
    pub d: String,
    pub p: String,
    pub q: String,
    pub dp: String,
    pub dq: String,
    pub qi: String,
}

#[derive(Clone)]
pub struct Signer {
    pub address: String,
    key: BlindedSigningKey<Sha256>,
}

impl Signer {
    pub fn from_file(path: &str) -> Result<Self> {
        let text = fs::read_to_string(path).unwrap();
        let wallet: Wallet = serde_json::from_str(&text).unwrap();
        Signer::from_wallet(wallet)
    }

    pub fn from_wallet(wallet: Wallet) -> Result<Self> {
        let n = base64_url_to_biguint(&wallet.n)?;
        let e = base64_url_to_biguint(&wallet.e)?;
        let d = base64_url_to_biguint(&wallet.d)?;
        let primes = vec![
            base64_url_to_biguint(&wallet.p)?,
            base64_url_to_biguint(&wallet.q)?,
        ];

        let private_key = RsaPrivateKey::from_components(n, e, d, primes)?;
        let key = BlindedSigningKey::<Sha256>::new(private_key);
        Ok(Self {
            key,
            address: wallet.n,
        })
    }

    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut rng = OsRng;
        Ok(self.key.sign_with_rng(&mut rng, data).to_vec())
    }
}

fn base64_url_to_biguint(b64: &str) -> Result<BigUint> {
    let bytes = URL_SAFE_NO_PAD.decode(b64)?;
    Ok(BigUint::from_bytes_be(&bytes))
}
