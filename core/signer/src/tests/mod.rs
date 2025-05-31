use aes_gcm::aead::OsRng;
use k256::ecdsa::SigningKey;

use crate::Password;

#[tokio::test]
async fn encode_decode_secret() -> anyhow::Result<()> {
    let pass = Password::new("qwerty123".into());
    let secret = SigningKey::random(&mut OsRng);
    let encoded_secret = pass.encode_signature(secret.clone())?;
    assert_eq!(Some(secret), pass.decode_signature(encoded_secret)?);
    Ok(())
}

#[tokio::test]
async fn try_to_decode_with_wrong_password() -> anyhow::Result<()> {
    let pass = Password::new("qwerty123".into());
    let wrong_pass = Password::new("qwerty1234".into());
    let secret = SigningKey::random(&mut OsRng);
    let encoded_secret = pass.encode_signature(secret.clone())?;
    assert_eq!(None, wrong_pass.decode_signature(encoded_secret)?);
    Ok(())
}
