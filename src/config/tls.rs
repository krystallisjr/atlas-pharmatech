///! TLS/HTTPS Configuration
///!
///! Provides secure transport layer configuration for production deployments.
///! Supports both development (self-signed) and production (Let's Encrypt) certificates.
///!
///! Security features:
///! - TLS 1.3 and TLS 1.2 only (no older versions)
///! - Strong cipher suites
///! - Certificate validation
///! - Automatic HTTPS redirect (production)

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub port: u16,
}

impl TlsConfig {
    /// Load TLS configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let enabled = env::var("TLS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        if !enabled {
            return Ok(Self {
                enabled: false,
                cert_path: PathBuf::new(),
                key_path: PathBuf::new(),
                port: 8080,
            });
        }

        let cert_path = env::var("TLS_CERT_PATH")
            .context("TLS_CERT_PATH must be set when TLS is enabled")?;
        let key_path = env::var("TLS_KEY_PATH")
            .context("TLS_KEY_PATH must be set when TLS is enabled")?;
        let port = env::var("TLS_PORT")
            .unwrap_or_else(|_| "8443".to_string())
            .parse()
            .context("Invalid TLS_PORT")?;

        Ok(Self {
            enabled: true,
            cert_path: PathBuf::from(cert_path),
            key_path: PathBuf::from(key_path),
            port,
        })
    }

    /// Build RustlsConfig for axum-server
    pub async fn build_rustls_config(&self) -> Result<RustlsConfig> {
        if !self.enabled {
            anyhow::bail!("TLS is not enabled");
        }

        // Validate certificate files exist
        if !self.cert_path.exists() {
            anyhow::bail!("TLS certificate not found at {:?}", self.cert_path);
        }
        if !self.key_path.exists() {
            anyhow::bail!("TLS private key not found at {:?}", self.key_path);
        }

        // Load certificates
        let config = RustlsConfig::from_pem_file(&self.cert_path, &self.key_path)
            .await
            .context("Failed to load TLS certificates")?;

        tracing::info!(
            "✅ TLS configured with certificate: {:?}",
            self.cert_path
        );

        Ok(config)
    }
}

/// Generate self-signed certificate for development
///
/// This function provides instructions for generating development certificates.
/// For production, use Let's Encrypt with certbot.
pub fn print_dev_cert_instructions() {
    println!("\n=== GENERATE DEVELOPMENT TLS CERTIFICATES ===\n");
    println!("Run the following command to create self-signed certificates:\n");
    println!("  mkdir -p certs");
    println!("  openssl req -x509 -newkey rsa:4096 \\");
    println!("    -keyout certs/key.pem \\");
    println!("    -out certs/cert.pem \\");
    println!("    -days 365 -nodes \\");
    println!("    -subj '/CN=localhost'\n");
    println!("Then add to .env:");
    println!("  TLS_ENABLED=true");
    println!("  TLS_CERT_PATH=./certs/cert.pem");
    println!("  TLS_KEY_PATH=./certs/key.pem");
    println!("  TLS_PORT=8443\n");
}

/// Production certificate setup instructions (Let's Encrypt)
pub fn print_production_cert_instructions(domain: &str) {
    println!("\n=== PRODUCTION TLS SETUP (Let's Encrypt) ===\n");
    println!("For production on DigitalOcean/AWS:\n");
    println!("1. Install certbot:");
    println!("   sudo apt-get update");
    println!("   sudo apt-get install certbot\n");
    println!("2. Generate certificate:");
    println!("   sudo certbot certonly --standalone -d {}\n", domain);
    println!("3. Update .env:");
    println!("   TLS_ENABLED=true");
    println!("   TLS_CERT_PATH=/etc/letsencrypt/live/{}/fullchain.pem", domain);
    println!("   TLS_KEY_PATH=/etc/letsencrypt/live/{}/privkey.pem", domain);
    println!("   TLS_PORT=443\n");
    println!("4. Set up auto-renewal:");
    println!("   sudo crontab -e");
    println!("   # Add: 0 0 1 * * certbot renew --quiet\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_disabled_by_default() {
        std::env::remove_var("TLS_ENABLED");
        let config = TlsConfig::from_env().unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_tls_config_validation() {
        std::env::set_var("TLS_ENABLED", "true");
        std::env::remove_var("TLS_CERT_PATH");

        let result = TlsConfig::from_env();
        assert!(result.is_err());

        std::env::remove_var("TLS_ENABLED");
    }
}
