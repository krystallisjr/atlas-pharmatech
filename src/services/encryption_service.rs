///! AES-256-GCM Encryption Service
///!
///! Provides authenticated encryption for sensitive data at rest.
///! Uses AES-256 in GCM mode (Galois/Counter Mode) which provides both
///! confidentiality and authenticity.
///!
///! Security properties:
///! - AES-256: 256-bit key strength
///! - GCM: Authenticated encryption (detects tampering)
///! - Unique nonce per encryption (prevents replay attacks)
///! - Constant-time operations (prevents timing attacks)

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::RngCore, SaltString};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore as _;
use sha2::{Sha256, Digest};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key")]
    InvalidKey,

    #[error("Invalid ciphertext format")]
    InvalidFormat,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
}

pub type Result<T> = std::result::Result<T, EncryptionError>;

/// Encryption service for sensitive data
///
/// Thread-safe, can be cloned and shared across threads
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create new encryption service from base64-encoded key
    ///
    /// Key must be exactly 32 bytes (256 bits) when decoded
    pub fn new(base64_key: &str) -> Result<Self> {
        let key_bytes = BASE64
            .decode(base64_key)
            .map_err(|e| EncryptionError::InvalidKey)?;

        if key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKey);
        }

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| EncryptionError::InvalidKey)?;

        Ok(Self { cipher })
    }

    /// Encrypt plaintext data
    ///
    /// Returns base64-encoded string: nonce(12 bytes) || ciphertext || tag(16 bytes)
    /// Format: "base64(nonce + encrypted_data + auth_tag)"
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        // Generate unique 96-bit nonce (12 bytes) - MUST be unique per encryption
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt plaintext with authentication
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Combine nonce + ciphertext for storage
        // We need the nonce for decryption, so we prepend it
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        // Encode as base64 for database storage
        Ok(BASE64.encode(&combined))
    }

    /// Decrypt ciphertext data
    ///
    /// Expects base64-encoded string containing nonce + encrypted_data + auth_tag
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        if ciphertext.is_empty() {
            return Ok(String::new());
        }

        // Decode from base64
        let combined = BASE64
            .decode(ciphertext)
            .map_err(|e| EncryptionError::InvalidFormat)?;

        // Must have at least nonce (12) + tag (16) = 28 bytes
        if combined.len() < 28 {
            return Err(EncryptionError::InvalidFormat);
        }

        // Split nonce and ciphertext
        let (nonce_bytes, encrypted_data) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt and verify authentication tag
        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // Convert to UTF-8 string
        String::from_utf8(plaintext_bytes)
            .map_err(|e| EncryptionError::DecryptionFailed("Invalid UTF-8".to_string()))
    }

    /// Encrypt optional string (for Option<String> fields)
    pub fn encrypt_optional(&self, plaintext: Option<&String>) -> Result<Option<String>> {
        match plaintext {
            Some(text) => Ok(Some(self.encrypt(text)?)),
            None => Ok(None),
        }
    }

    /// Decrypt optional string
    pub fn decrypt_optional(&self, ciphertext: Option<&String>) -> Result<Option<String>> {
        match ciphertext {
            Some(text) => Ok(Some(self.decrypt(text)?)),
            None => Ok(None),
        }
    }

    /// Generate a new 256-bit encryption key
    ///
    /// Returns base64-encoded key suitable for environment variables
    pub fn generate_key() -> String {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        BASE64.encode(&key_bytes)
    }

    /// Generate SHA-256 hash for searchable fields
    ///
    /// Use this for creating searchable indexes of encrypted data.
    /// The hash allows fast lookups without exposing plaintext.
    ///
    /// # Example
    /// ```
    /// let email_hash = EncryptionService::hash_for_lookup("user@example.com");
    /// // Store email_hash in database for fast queries
    /// // WHERE email_hash = $1
    /// ```
    pub fn hash_for_lookup(plaintext: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(plaintext.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Encrypt binary data (for files like Excel, images, etc.)
    ///
    /// Returns base64-encoded string: nonce(12 bytes) || ciphertext || tag(16 bytes)
    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        // Generate unique 96-bit nonce (12 bytes)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt binary data with authentication
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Combine nonce + ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// Decrypt to binary data (for files like Excel, images, etc.)
    ///
    /// Returns the original binary data
    pub fn decrypt_bytes(&self, ciphertext: &str) -> Result<Vec<u8>> {
        if ciphertext.is_empty() {
            return Ok(Vec::new());
        }

        // Decode from base64
        let combined = BASE64
            .decode(ciphertext)
            .map_err(|_| EncryptionError::InvalidFormat)?;

        // Must have at least nonce (12) + tag (16) = 28 bytes
        if combined.len() < 28 {
            return Err(EncryptionError::InvalidFormat);
        }

        // Split nonce and ciphertext
        let (nonce_bytes, encrypted_data) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt and verify authentication tag
        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext_bytes)
    }
}

/// Generate a new encryption key (for initial setup)
pub fn generate_encryption_key() -> String {
    EncryptionService::generate_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(&key).unwrap();

        let plaintext = "sensitive data";
        let ciphertext = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_ne!(plaintext, ciphertext); // Ensure it's actually encrypted
    }

    #[test]
    fn test_empty_string() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(&key).unwrap();

        let ciphertext = service.encrypt("").unwrap();
        let decrypted = service.decrypt(&ciphertext).unwrap();

        assert_eq!("", decrypted);
    }

    #[test]
    fn test_unique_nonces() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(&key).unwrap();

        let plaintext = "same data";
        let ct1 = service.encrypt(plaintext).unwrap();
        let ct2 = service.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(ct1, ct2);

        // But both should decrypt to same plaintext
        assert_eq!(service.decrypt(&ct1).unwrap(), plaintext);
        assert_eq!(service.decrypt(&ct2).unwrap(), plaintext);
    }

    #[test]
    fn test_tampered_ciphertext() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(&key).unwrap();

        let plaintext = "sensitive data";
        let mut ciphertext = service.encrypt(plaintext).unwrap();

        // Tamper with the ciphertext
        ciphertext.push('X');

        // Decryption should fail (authentication check)
        assert!(service.decrypt(&ciphertext).is_err());
    }

    #[test]
    fn test_optional_encryption() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(&key).unwrap();

        let some_data = Some(String::from("data"));
        let encrypted = service.encrypt_optional(some_data.as_ref()).unwrap();
        let decrypted = service.decrypt_optional(encrypted.as_ref()).unwrap();

        assert_eq!(some_data, decrypted);

        let none_data: Option<String> = None;
        let encrypted = service.encrypt_optional(none_data.as_ref()).unwrap();
        assert!(encrypted.is_none());
    }
}
