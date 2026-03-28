use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

use crate::SecurityError;

pub struct PinManager;

impl PinManager {
    /// Hash a password/PIN using Argon2id. Returns the PHC string (contains salt + hash).
    pub fn hash_password(password: &str) -> Result<String, SecurityError> {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(65536, 3, 4, None)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password/PIN against a stored PHC hash string.
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, SecurityError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_correct() {
        let hash = PinManager::hash_password("mySecurePin123").unwrap();
        assert!(PinManager::verify_password("mySecurePin123", &hash).unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let hash = PinManager::hash_password("correct").unwrap();
        assert!(!PinManager::verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_hash_is_unique_per_call() {
        let h1 = PinManager::hash_password("same").unwrap();
        let h2 = PinManager::hash_password("same").unwrap();
        assert_ne!(h1, h2); // Different salts
    }

    #[test]
    fn test_empty_password() {
        let hash = PinManager::hash_password("").unwrap();
        assert!(PinManager::verify_password("", &hash).unwrap());
        assert!(!PinManager::verify_password("x", &hash).unwrap());
    }
}
