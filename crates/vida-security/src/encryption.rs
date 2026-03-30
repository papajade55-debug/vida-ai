use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::SecurityError;

/// Encrypt data with AES-256-GCM. Returns nonce (12 bytes) + ciphertext as base64.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<String, SecurityError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(combined))
}

/// Decrypt base64(nonce + ciphertext) with AES-256-GCM.
pub fn decrypt(key: &[u8; 32], encoded: &str) -> Result<Vec<u8>, SecurityError> {
    use base64::Engine;
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    if combined.len() < 13 {
        return Err(SecurityError::EncryptionFailed(
            "Data too short".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"Hello, Vida AI!";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_decrypt_short_data_fails() {
        let key = [42u8; 32];
        use base64::Engine;
        let short = base64::engine::general_purpose::STANDARD.encode([0u8; 5]);
        assert!(decrypt(&key, &short).is_err());
    }
}
