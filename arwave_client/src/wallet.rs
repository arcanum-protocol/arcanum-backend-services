use std::str::FromStr;

use base64::{prelude::BASE64_STANDARD, Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rsa::{rand_core::OsRng, pss::BlindedSigningKey, BigUint, Pss, RsaPrivateKey, sha2::Sha256, signature::{RandomizedSigner, SignatureEncoding}};
use serde::Deserialize;
use anyhow::{anyhow, Result};

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

impl Wallet {
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let n = base64_url_to_biguint(&self.n)?;
        let e = base64_url_to_biguint(&self.e)?;
        let d = base64_url_to_biguint(&self.d)?;
        let primes = vec![
            base64_url_to_biguint(&self.p)?,
            base64_url_to_biguint(&self.q)?,
        ];
        let exponents = vec![
            base64_url_to_biguint(&self.dp)?,
            base64_url_to_biguint(&self.dq)?,
        ];
        let coefficient = base64_url_to_biguint(&self.qi)?;
        let private_key = RsaPrivateKey::from_components(n, e, d, primes)?;
        let mut rng = OsRng;
        let key = BlindedSigningKey::<Sha256>::new(private_key);
        Ok(key.sign_with_rng(&mut rng, data).to_vec())
    }
}


fn base64_url_to_biguint(b64: &str) -> Result<BigUint> {
    let bytes = URL_SAFE_NO_PAD.decode(b64)?;
    Ok(BigUint::from_bytes_be(&bytes))
}

