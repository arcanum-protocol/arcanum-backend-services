#[cfg(test)]
pub mod tests;

use aes_gcm::aead::rand_core;
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::Aead;
use aes_gcm::AeadCore;
use aes_gcm::Aes256Gcm;
use aes_gcm::KeyInit;
use aes_gcm::Nonce;

use argon2::Argon2;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;

use k256::ecdsa::{Signature, SigningKey};
use k256::Scalar;
use rand_core::OsRng;

use tokio::fs;
use tokio::io::AsyncReadExt;

pub mod price;

pub struct Password {
    inner: String,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct EnctryptedSecretKey {
    password_salt: Vec<u8>,
    encryption_salt: Vec<u8>,
    encrypted_data: Vec<u8>,
}

impl Password {
    pub fn new(password: String) -> Self {
        Self { inner: password }
    }

    pub async fn load_password(password_path: &str) -> anyhow::Result<Self> {
        let mut file = fs::File::open(password_path).await?;
        let mut inner = String::default();
        file.read_to_string(&mut inner).await?;
        Ok(Self { inner })
    }

    pub fn decode_signature(
        &self,
        secret: EnctryptedSecretKey,
    ) -> anyhow::Result<Option<SigningKey>> {
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(self.inner.as_bytes(), &secret.password_salt, &mut key)
            .unwrap();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();

        let nonce = Nonce::from_slice(&secret.encryption_salt);
        match cipher.decrypt(nonce, secret.encrypted_data.as_ref()) {
            Ok(b) => Ok(Some(SigningKey::from_slice(&b)?)),
            Err(_) => Ok(None),
        }
    }

    pub fn encode_signature(&self, pkey: SigningKey) -> anyhow::Result<EnctryptedSecretKey> {
        let mut password_salt = [0u8; 64];
        rand_core::OsRng.fill_bytes(&mut password_salt);

        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(self.inner.as_bytes(), &password_salt, &mut key)
            .unwrap();

        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, pkey.to_bytes().as_slice()).unwrap();
        Ok(EnctryptedSecretKey {
            password_salt: password_salt.to_vec(),
            encryption_salt: nonce.to_vec(),
            encrypted_data: ciphertext.to_vec(),
        })
    }

    pub async fn retreive_or_generate_pkey(
        self,
        secret_key_path: &str,
    ) -> anyhow::Result<SigningKey> {
        let mut file = fs::File::open(secret_key_path).await?;
        let mut secret_key_bytes = Vec::new();
        file.read_to_end(&mut secret_key_bytes).await?;

        if secret_key_bytes.is_empty() {
            let k = SigningKey::random(&mut OsRng);
            let encrypted = self.encode_signature(k.clone())?;

            let mut encrypted_bytes = Vec::new();
            encrypted.serialize(&mut encrypted_bytes)?;
            fs::write(secret_key_path, encrypted_bytes).await?;

            Ok(k)
        } else {
            let secret = EnctryptedSecretKey::deserialize(&mut secret_key_bytes.as_slice())?;
            self.decode_signature(secret)
                .transpose()
                .expect("Password not valid")
        }
    }
}

/// Simplified signature combination (for demonstration only - not cryptographically secure)
pub fn combine_signatures(sigs: &[Signature]) -> anyhow::Result<Signature> {
    let (r, s) = sigs.iter().fold((Scalar::ZERO, Scalar::ZERO), |acc, s| {
        let (r, s) = s.split_scalars();
        (*r, acc.1.add(&s))
    });

    Signature::from_scalars(r, s).map_err(Into::into)
}
