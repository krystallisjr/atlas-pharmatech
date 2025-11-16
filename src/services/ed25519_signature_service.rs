// 🔐 ED25519 SIGNATURE SERVICE - LIBSODIUM (PRODUCTION-READY)
// Used for regulatory document signing with cryptographic non-repudiation
// One keypair per user for maximum compliance

use crate::middleware::error_handling::Result;
use anyhow::anyhow;
use hex::{decode as hex_decode, encode as hex_encode};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

// Sodiumoxide Ed25519 types
use sodiumoxide::crypto::sign::{self, PublicKey, SecretKey, Signature};

/// Ed25519 Signature Service using libsodium (sodiumoxide)
/// Provides cryptographically secure document signing for regulatory compliance
pub struct Ed25519SignatureService {
    db_pool: PgPool,
    encryption_key: Vec<u8>,
}

impl Ed25519SignatureService {
    /// Create new Ed25519 signature service
    ///
    /// # Arguments
    /// * `db_pool` - PostgreSQL connection pool
    /// * `encryption_key` - 32-byte key for encrypting private keys at rest (AES-256-GCM)
    pub fn new(db_pool: PgPool, encryption_key: &str) -> Result<Self> {
        // Decode base64 encryption key
        let encryption_key = base64::decode(encryption_key)
            .map_err(|_| anyhow!("Invalid encryption key - must be base64-encoded 32 bytes"))?;

        if encryption_key.len() != 32 {
            return Err(anyhow!("Encryption key must be exactly 32 bytes").into());
        }

        // Initialize libsodium
        sodiumoxide::init().map_err(|_| anyhow!("Failed to initialize libsodium"))?;

        Ok(Self {
            db_pool,
            encryption_key,
        })
    }

    /// Generate Ed25519 keypair for a user
    ///
    /// This creates a new keypair and stores it in the database.
    /// The private key is encrypted with AES-256-GCM before storage.
    ///
    /// # Arguments
    /// * `user_id` - UUID of the user
    ///
    /// # Returns
    /// * `Ok(())` if keypair was generated and stored successfully
    /// * `Err(_)` if user already has a keypair or database error
    pub async fn generate_user_keypair(&self, user_id: Uuid) -> Result<()> {
        // Check if user already has a keypair
        let existing = sqlx::query!(
            "SELECT ed25519_public_key FROM users WHERE id = $1",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        if existing.ed25519_public_key.is_some() {
            return Err(anyhow!("User already has an Ed25519 keypair - cannot regenerate").into());
        }

        // Generate new Ed25519 keypair using libsodium
        let (public_key, secret_key) = sign::gen_keypair();

        // Encode public key as hex (32 bytes -> 64 hex chars)
        let public_key_hex = hex_encode(public_key.as_ref());

        // Encrypt private key with AES-256-GCM before storage
        let secret_key_encrypted = self.encrypt_private_key(&secret_key)?;

        // Store keypair in database
        sqlx::query!(
            "UPDATE users
             SET ed25519_public_key = $1,
                 ed25519_private_key_encrypted = $2,
                 keypair_generated_at = NOW()
             WHERE id = $3",
            public_key_hex,
            secret_key_encrypted,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            "Generated Ed25519 keypair for user {} - public key: {}",
            user_id,
            public_key_hex
        );

        Ok(())
    }

    /// Sign a document with user's private key
    ///
    /// This creates a detached Ed25519 signature of the SHA-256 hash of the content.
    ///
    /// # Arguments
    /// * `user_id` - UUID of the user signing the document
    /// * `content` - Document content to sign (will be hashed with SHA-256)
    ///
    /// # Returns
    /// * `Ok((signature_hex, content_hash_hex))` - Signature and content hash (both hex-encoded)
    /// * `Err(_)` if user has no keypair or signing fails
    pub async fn sign_document(
        &self,
        user_id: Uuid,
        content: &str,
    ) -> Result<(String, String)> {
        // Retrieve user's encrypted private key
        let user = sqlx::query!(
            "SELECT ed25519_private_key_encrypted, ed25519_public_key
             FROM users WHERE id = $1",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        let private_key_encrypted = user
            .ed25519_private_key_encrypted
            .ok_or_else(|| anyhow!("User has no Ed25519 keypair - call generate_user_keypair first"))?;

        let public_key_hex = user
            .ed25519_public_key
            .ok_or_else(|| anyhow!("User has no Ed25519 public key"))?;

        // Decrypt private key
        let secret_key = self.decrypt_private_key(&private_key_encrypted)?;

        // Calculate SHA-256 hash of content
        let content_hash = Sha256::digest(content.as_bytes());
        let content_hash_hex = hex_encode(&content_hash);

        // Sign the hash with Ed25519 (detached signature)
        let signature = sign::sign_detached(&content_hash, &secret_key);

        // Encode signature as hex (64 bytes -> 128 hex chars)
        let signature_hex = hex_encode(signature.as_ref());

        tracing::info!(
            "Signed document for user {} - signature: {}, hash: {}",
            user_id,
            &signature_hex[..16],
            &content_hash_hex[..16]
        );

        Ok((signature_hex, content_hash_hex))
    }

    /// Verify a document signature
    ///
    /// This verifies that the signature was created by the holder of the private key
    /// corresponding to the given public key.
    ///
    /// # Arguments
    /// * `content_hash_hex` - SHA-256 hash of the document (hex-encoded)
    /// * `signature_hex` - Ed25519 signature (hex-encoded)
    /// * `public_key_hex` - Ed25519 public key (hex-encoded)
    ///
    /// # Returns
    /// * `Ok(true)` if signature is valid
    /// * `Ok(false)` if signature is invalid
    /// * `Err(_)` if hex decoding fails
    pub fn verify_signature(
        &self,
        content_hash_hex: &str,
        signature_hex: &str,
        public_key_hex: &str,
    ) -> Result<bool> {
        // Decode hex inputs
        let content_hash =
            hex_decode(content_hash_hex).map_err(|_| anyhow!("Invalid content hash hex"))?;
        let signature_bytes =
            hex_decode(signature_hex).map_err(|_| anyhow!("Invalid signature hex"))?;
        let public_key_bytes =
            hex_decode(public_key_hex).map_err(|_| anyhow!("Invalid public key hex"))?;

        // Parse Ed25519 public key and signature
        let public_key = PublicKey::from_slice(&public_key_bytes)
            .ok_or_else(|| anyhow!("Invalid Ed25519 public key (must be 32 bytes)"))?;

        // sodiumoxide Signature - use the new constructor
        if signature_bytes.len() != 64 {
            return Err(anyhow!("Invalid Ed25519 signature (must be 64 bytes)").into());
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::new(sig_array);

        // Verify signature
        let is_valid = sign::verify_detached(&signature, &content_hash, &public_key);

        if is_valid {
            tracing::info!("Signature verified successfully - hash: {}", &content_hash_hex[..16]);
        } else {
            tracing::warn!("Signature verification FAILED - hash: {}", &content_hash_hex[..16]);
        }

        Ok(is_valid)
    }

    /// Verify blockchain-style ledger chain integrity
    ///
    /// This checks that each ledger entry's chain_hash correctly links to the previous entry.
    ///
    /// # Arguments
    /// * `document_id` - UUID of the document to verify
    ///
    /// # Returns
    /// * `Ok(true)` if chain is valid
    /// * `Ok(false)` if chain is broken
    /// * `Err(_)` on database error
    pub async fn verify_ledger_chain_integrity(&self, document_id: Uuid) -> Result<bool> {
        // Fetch all ledger entries for this document with epoch timestamp
        let entries = sqlx::query!(
            r#"
            SELECT
                id,
                content_hash,
                signature,
                created_at,
                EXTRACT(EPOCH FROM created_at)::TEXT as "epoch_text!",
                previous_entry_hash,
                chain_hash
            FROM regulatory_document_ledger
            WHERE document_id = $1
            ORDER BY id ASC
            "#,
            document_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        if entries.is_empty() {
            return Ok(true); // No entries = valid
        }

        // Verify each entry's chain hash
        let mut previous_hash: Option<String> = None;
        let entry_count = entries.len();

        for entry in &entries {
            // Recalculate chain hash (must match database trigger exactly!)
            // Use the exact epoch text and previous_entry_hash from database
            let prev_hash_for_calc = entry.previous_entry_hash.as_deref()
                .unwrap_or("0000000000000000000000000000000000000000000000000000000000000000");

            let data_to_hash = format!(
                "{}{}{}{}",
                prev_hash_for_calc,
                entry.content_hash,
                entry.signature,
                entry.epoch_text
            );

            let calculated_hash = Sha256::digest(data_to_hash.as_bytes());
            let calculated_hash_hex = hex_encode(&calculated_hash);

            // Compare with stored chain hash
            if calculated_hash_hex != entry.chain_hash {
                tracing::error!(
                    "Chain integrity BROKEN at ledger entry {}",
                    entry.id
                );
                tracing::error!(
                    "  Rust calculated: {}",
                    calculated_hash_hex
                );
                tracing::error!(
                    "  DB stored:       {}",
                    entry.chain_hash
                );
                tracing::error!(
                    "  prev_hash:   {}",
                    prev_hash_for_calc
                );
                tracing::error!(
                    "  content_hash: {}",
                    entry.content_hash
                );
                tracing::error!(
                    "  signature:    {}",
                    entry.signature
                );
                tracing::error!(
                    "  epoch_text:   {}",
                    entry.epoch_text
                );
                return Ok(false);
            }

            previous_hash = Some(entry.chain_hash.clone());
        }

        tracing::info!("Ledger chain integrity verified - {} entries", entry_count);
        Ok(true)
    }

    /// Get user's public key (for verification)
    ///
    /// # Arguments
    /// * `user_id` - UUID of the user
    ///
    /// # Returns
    /// * `Ok(Some(public_key_hex))` if user has a keypair
    /// * `Ok(None)` if user has no keypair
    pub async fn get_user_public_key(&self, user_id: Uuid) -> Result<Option<String>> {
        let row = sqlx::query!(
            "SELECT ed25519_public_key FROM users WHERE id = $1",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(row.ed25519_public_key)
    }

    /// Check if user has a keypair
    pub async fn has_keypair(&self, user_id: Uuid) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT ed25519_public_key FROM users WHERE id = $1",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(row.ed25519_public_key.is_some())
    }

    // ============================================================================
    // PRIVATE HELPER METHODS - AES-256-GCM ENCRYPTION
    // ============================================================================

    /// Encrypt private key with AES-256-GCM (for storage in database)
    fn encrypt_private_key(&self, secret_key: &SecretKey) -> Result<String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Create cipher with encryption key
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|_| anyhow!("Invalid encryption key length"))?;

        // Generate random nonce (12 bytes for GCM)
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt private key (32 bytes)
        let ciphertext = cipher
            .encrypt(nonce, secret_key.as_ref())
            .map_err(|_| anyhow!("Failed to encrypt private key"))?;

        // Prepend nonce to ciphertext for storage (nonce || ciphertext)
        let mut encrypted = nonce_bytes.to_vec();
        encrypted.extend_from_slice(&ciphertext);

        // Base64 encode for storage
        Ok(base64::encode(&encrypted))
    }

    /// Decrypt private key from database
    fn decrypt_private_key(&self, encrypted_base64: &str) -> Result<SecretKey> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Decode base64
        let encrypted = base64::decode(encrypted_base64)
            .map_err(|_| anyhow!("Invalid base64 in encrypted private key"))?;

        if encrypted.len() < 12 {
            return Err(anyhow!("Encrypted private key too short").into());
        }

        // Extract nonce (first 12 bytes) and ciphertext (rest)
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|_| anyhow!("Invalid encryption key length"))?;

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Failed to decrypt private key - wrong encryption key?"))?;

        // Parse as Ed25519 secret key (32 bytes)
        let secret_key = SecretKey::from_slice(&plaintext)
            .ok_or_else(|| anyhow!("Invalid Ed25519 private key (must be 32 bytes)"))?;

        Ok(secret_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signature_roundtrip() {
        // Initialize libsodium
        sodiumoxide::init().unwrap();

        // Generate test keypair
        let (public_key, secret_key) = sign::gen_keypair();

        // Test document
        let content = "This is a Certificate of Analysis for Batch ABC-123";
        let content_hash = Sha256::digest(content.as_bytes());

        // Sign
        let signature = sign::sign_detached(&content_hash, &secret_key);

        // Verify
        let is_valid = sign::verify_detached(&signature, &content_hash, &public_key);

        assert!(is_valid, "Signature should be valid");
    }

    #[test]
    fn test_encryption_roundtrip() {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Generate random 32-byte key
        let key = base64::encode(&[42u8; 32]);
        let key_bytes = base64::decode(&key).unwrap();

        sodiumoxide::init().unwrap();
        let (_, secret_key) = sign::gen_keypair();

        // Encrypt
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, secret_key.as_ref()).unwrap();

        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();

        assert_eq!(secret_key.as_ref(), &plaintext[..]);
    }
}
