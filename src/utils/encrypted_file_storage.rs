/// Encrypted file storage utility for secure file uploads
///
/// SECURITY: All files are encrypted at rest using AES-256-GCM
/// - Each file gets a unique nonce (stored as file prefix)
/// - Authenticated encryption prevents tampering
/// - SHA256 hash computed on plaintext for integrity verification
///
/// File format: [12-byte nonce][encrypted data with auth tag]

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Write, Read};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::middleware::error_handling::{AppError, Result};
use crate::services::EncryptionService;

pub struct EncryptedFileStorage {
    base_path: PathBuf,
    encryption: EncryptionService,
}

impl EncryptedFileStorage {
    pub fn new(base_path: impl AsRef<Path>, encryption_key: &str) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create base directory if it doesn't exist
        if !base_path.exists() {
            fs::create_dir_all(&base_path)
                .map_err(|e| AppError::Internal(
                    anyhow::anyhow!("Failed to create storage directory: {}", e)
                ))?;
        }

        let encryption = EncryptionService::new(encryption_key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to initialize encryption: {}", e)))?;

        Ok(Self { base_path, encryption })
    }

    /// Save encrypted file to disk and return the file path and SHA256 hash of PLAINTEXT
    ///
    /// Returns: (relative_path, plaintext_hash)
    pub fn save_encrypted_file(
        &self,
        session_id: Uuid,
        filename: &str,
        plaintext_data: &[u8],
    ) -> Result<(String, String)> {
        // Calculate SHA256 hash of PLAINTEXT (for integrity verification)
        let mut hasher = Sha256::new();
        hasher.update(plaintext_data);
        let plaintext_hash = format!("{:x}", hasher.finalize());

        // Encrypt the binary file data (preserves Excel/binary files correctly)
        let encrypted_data = self.encryption.encrypt_bytes(plaintext_data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Encryption failed: {}", e)))?;

        // Create session directory
        let session_dir = self.base_path.join(session_id.to_string());
        fs::create_dir_all(&session_dir)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to create session directory: {}", e)
            ))?;

        // Sanitize filename and add .enc extension
        let safe_filename = format!("{}.enc", sanitize_filename(filename));
        let file_path = session_dir.join(&safe_filename);

        // Write encrypted data to disk
        let mut file = fs::File::create(&file_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to create file: {}", e)
            ))?;

        file.write_all(encrypted_data.as_bytes())
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to write encrypted file: {}", e)
            ))?;

        // Return relative path from base
        let relative_path = file_path
            .strip_prefix(&self.base_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        tracing::info!(
            "🔒 File encrypted and saved: {} (plaintext_hash: {})",
            relative_path,
            &plaintext_hash[..8]
        );

        Ok((relative_path, plaintext_hash))
    }

    /// Read and decrypt file from disk
    pub fn read_encrypted_file(&self, relative_path: &str) -> Result<Vec<u8>> {
        let full_path = self.base_path.join(relative_path);

        // Read encrypted data (base64-encoded string)
        let mut file = fs::File::open(&full_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to open file {}: {}", relative_path, e)
            ))?;

        let mut encrypted_data = String::new();
        file.read_to_string(&mut encrypted_data)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to read encrypted file: {}", e)
            ))?;

        // Decrypt to binary data (preserves Excel/binary files correctly)
        let plaintext_bytes = self.encryption.decrypt_bytes(&encrypted_data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Decryption failed: {}", e)))?;

        tracing::info!("🔓 File decrypted: {}", relative_path);

        Ok(plaintext_bytes)
    }

    /// Verify file integrity by comparing hash
    pub fn verify_file(&self, relative_path: &str, expected_hash: &str) -> Result<bool> {
        let plaintext_data = self.read_encrypted_file(relative_path)?;

        let mut hasher = Sha256::new();
        hasher.update(&plaintext_data);
        let actual_hash = format!("{:x}", hasher.finalize());

        Ok(actual_hash == expected_hash)
    }

    /// Delete encrypted file and its session directory if empty
    pub fn delete_file(&self, relative_path: &str) -> Result<()> {
        let full_path = self.base_path.join(relative_path);

        if full_path.exists() {
            fs::remove_file(&full_path)
                .map_err(|e| AppError::Internal(
                    anyhow::anyhow!("Failed to delete file: {}", e)
                ))?;

            // Try to remove parent directory if empty
            if let Some(parent) = full_path.parent() {
                let _ = fs::remove_dir(parent); // Ignore error if not empty
            }

            tracing::info!("🗑️  Encrypted file deleted: {}", relative_path);
        }

        Ok(())
    }

    /// Clean up old encrypted files (older than days specified)
    pub fn cleanup_old_files(&self, days: u64) -> Result<usize> {
        use std::time::{SystemTime, Duration};

        let cutoff = SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60);
        let mut deleted_count = 0;

        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified < cutoff {
                            if let Ok(_) = fs::remove_dir_all(entry.path()) {
                                deleted_count += 1;
                                tracing::info!("🧹 Cleaned up old encrypted files: {:?}", entry.path());
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// Migrate existing plaintext file to encrypted format
    ///
    /// This is useful for migrating existing uploads to encrypted storage
    pub fn migrate_plaintext_file(
        &self,
        plaintext_path: &str,
        session_id: Uuid,
        filename: &str,
    ) -> Result<(String, String)> {
        // Read plaintext file
        let full_path = self.base_path.join(plaintext_path);
        let plaintext_data = fs::read(&full_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to read plaintext file: {}", e)
            ))?;

        // Save as encrypted
        let (encrypted_path, hash) = self.save_encrypted_file(session_id, filename, &plaintext_data)?;

        // Delete original plaintext file
        fs::remove_file(&full_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to delete plaintext file: {}", e)
            ))?;

        tracing::info!("♻️  Migrated plaintext file to encrypted: {} -> {}", plaintext_path, encrypted_path);

        Ok((encrypted_path, hash))
    }
}

/// Sanitize filename to prevent directory traversal
fn sanitize_filename(filename: &str) -> String {
    filename
        .replace("..", "")
        .replace("/", "_")
        .replace("\\", "_")
        .chars()
        .take(255)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_storage() -> (EncryptedFileStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        // Generate test encryption key (base64 encoded 32 bytes)
        let test_key = base64::engine::general_purpose::STANDARD
            .encode(&[0u8; 32]);
        let storage = EncryptedFileStorage::new(temp_dir.path(), &test_key).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_encrypt_decrypt_file() {
        let (storage, _temp_dir) = setup_test_storage();
        let session_id = Uuid::new_v4();
        let test_data = b"This is sensitive file content!";

        // Encrypt and save
        let (path, hash) = storage
            .save_encrypted_file(session_id, "test.txt", test_data)
            .unwrap();

        // Read and decrypt
        let decrypted = storage.read_encrypted_file(&path).unwrap();

        assert_eq!(test_data, &decrypted[..]);
        assert!(storage.verify_file(&path, &hash).unwrap());
    }

    #[test]
    fn test_file_integrity_verification() {
        let (storage, _temp_dir) = setup_test_storage();
        let session_id = Uuid::new_v4();
        let test_data = b"Important data";

        let (path, hash) = storage
            .save_encrypted_file(session_id, "data.bin", test_data)
            .unwrap();

        // Correct hash should verify
        assert!(storage.verify_file(&path, &hash).unwrap());

        // Wrong hash should fail
        let wrong_hash = "0".repeat(64);
        assert!(!storage.verify_file(&path, &wrong_hash).unwrap());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "_.._.._.._etc_passwd");
        assert_eq!(sanitize_filename("normal_file.csv"), "normal_file.csv");
        assert_eq!(sanitize_filename("file/with\\slashes.txt"), "file_with_slashes.txt");
    }

    #[test]
    fn test_delete_encrypted_file() {
        let (storage, _temp_dir) = setup_test_storage();
        let session_id = Uuid::new_v4();
        let test_data = b"Delete me";

        let (path, _hash) = storage
            .save_encrypted_file(session_id, "delete.txt", test_data)
            .unwrap();

        // File should exist
        assert!(storage.read_encrypted_file(&path).is_ok());

        // Delete file
        storage.delete_file(&path).unwrap();

        // File should not exist
        assert!(storage.read_encrypted_file(&path).is_err());
    }
}
