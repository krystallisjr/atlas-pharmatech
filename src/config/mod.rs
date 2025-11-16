pub mod tls;

use std::env;
use anyhow::Result;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub ssl_mode: String,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()?,
            username: env::var("DATABASE_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: env::var("DATABASE_PASSWORD")?,
            database: env::var("DATABASE_NAME").unwrap_or_else(|_| "atlas_pharma".to_string()),
            ssl_mode: env::var("DATABASE_SSL_MODE").unwrap_or_else(|_| "prefer".to_string()),
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.username, self.password, self.host, self.port, self.database, self.ssl_mode
        )
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub jwt_secret: String,
    pub encryption_key: String,
    pub server_host: String,
    pub server_port: u16,
    pub cors_origins: Vec<String>,
    pub database_pool: PgPool,
    pub file_storage_path: String,
}

impl AppConfig {
    pub async fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let database_config = DatabaseConfig::from_env()?;
        let database_pool = sqlx::PgPool::connect(&database_config.connection_string()).await?;

        Ok(Self {
            database: database_config,
            jwt_secret: env::var("JWT_SECRET")?,
            encryption_key: env::var("ENCRYPTION_KEY")?,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            cors_origins,
            database_pool,
            file_storage_path: env::var("FILE_STORAGE_PATH")
                .unwrap_or_else(|_| "./uploads".to_string()),
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}