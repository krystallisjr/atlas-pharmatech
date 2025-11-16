/// File storage utility for AI import system
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::middleware::error_handling::{AppError, Result};

pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create base directory if it doesn't exist
        if !base_path.exists() {
            fs::create_dir_all(&base_path)
                .map_err(|e| AppError::Internal(
                    anyhow::anyhow!("Failed to create storage directory: {}", e)
                ))?;
        }

        Ok(Self { base_path })
    }

    /// Save file to disk and return the file path and SHA256 hash
    pub fn save_file(
        &self,
        session_id: Uuid,
        filename: &str,
        data: &[u8],
    ) -> Result<(String, String)> {
        // Calculate SHA256 hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());

        // Create session directory
        let session_dir = self.base_path.join(session_id.to_string());
        fs::create_dir_all(&session_dir)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to create session directory: {}", e)
            ))?;

        // Sanitize filename
        let safe_filename = sanitize_filename(filename);
        let file_path = session_dir.join(&safe_filename);

        // Write file to disk
        let mut file = fs::File::create(&file_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to create file: {}", e)
            ))?;

        file.write_all(data)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to write file: {}", e)
            ))?;

        // Return relative path from base
        let relative_path = file_path
            .strip_prefix(&self.base_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        Ok((relative_path, hash))
    }

    /// Read file from disk
    pub fn read_file(&self, relative_path: &str) -> Result<Vec<u8>> {
        let full_path = self.base_path.join(relative_path);

        fs::read(&full_path)
            .map_err(|e| AppError::Internal(
                anyhow::anyhow!("Failed to read file {}: {}", relative_path, e)
            ))
    }

    /// Delete file and its session directory if empty
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
        }

        Ok(())
    }

    /// Clean up old files (older than days specified)
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
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
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

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "_.._.._.._etc_passwd");
        assert_eq!(sanitize_filename("normal_file.csv"), "normal_file.csv");
        assert_eq!(sanitize_filename("file/with\\slashes.txt"), "file_with_slashes.txt");
    }
}
