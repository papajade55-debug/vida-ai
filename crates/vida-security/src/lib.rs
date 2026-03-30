pub mod encryption;
pub mod keychain;
pub mod pin;

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Keychain access: {0}")]
    KeychainAccess(String),
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    #[error("Invalid PIN")]
    InvalidPin,
    #[error("Hashing failed: {0}")]
    HashingFailed(String),
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
}
