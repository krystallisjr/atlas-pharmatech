use sqlx::{PgPool, query, Row};
use uuid::Uuid;
use chrono::Utc;
use anyhow::anyhow;
use crate::models::user::{User, CreateUserRequest, UpdateUserRequest};
use crate::middleware::error_handling::{Result, AppError};
use crate::services::encryption_service::EncryptionService;

pub struct UserRepository {
    pool: PgPool,
    encryption: EncryptionService,
}

impl UserRepository {
    pub fn new(pool: PgPool, encryption_key: &str) -> Result<Self> {
        let encryption = EncryptionService::new(encryption_key)?;
        Ok(Self { pool, encryption })
    }

    pub async fn create(&self, request: &CreateUserRequest, password_hash: &str) -> Result<User> {
        // 🔒 PRODUCTION ENCRYPTION: Hash for lookup + Encrypt for storage
        let email_hash = EncryptionService::hash_for_lookup(&request.email);
        let email_encrypted = self.encryption.encrypt(&request.email)
            .map_err(|e| AppError::Internal(anyhow!("Encryption failed: {:?}", e)))?;
        let contact_person_encrypted = self.encryption.encrypt(&request.contact_person)
            .map_err(|e| AppError::Internal(anyhow!("Encryption failed: {:?}", e)))?;
        let phone_encrypted = match &request.phone {
            Some(s) => Some(self.encryption.encrypt(s).map_err(|e| AppError::Internal(anyhow!("Encryption failed: {:?}", e)))?),
            None => None,
        };
        let address_encrypted = match &request.address {
            Some(s) => Some(self.encryption.encrypt(s).map_err(|e| AppError::Internal(anyhow!("Encryption failed: {:?}", e)))?),
            None => None,
        };
        let license_number_encrypted = match &request.license_number {
            Some(s) => Some(self.encryption.encrypt(s).map_err(|e| AppError::Internal(anyhow!("Encryption failed: {:?}", e)))?),
            None => None,
        };

        let row = query(
            r#"
            INSERT INTO users (
                email, password_hash, company_name, contact_person, phone, address, license_number,
                email_hash, email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, password_hash, company_name, is_verified, created_at, updated_at,
                      email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted
            "#
        )
        .bind(&request.email)  // Temporary: for backwards compat during migration
        .bind(password_hash)
        .bind(&request.company_name)
        .bind(&request.contact_person)  // Temporary: for backwards compat
        .bind(&request.phone)  // Temporary: for backwards compat
        .bind(&request.address)  // Temporary: for backwards compat
        .bind(&request.license_number)  // Temporary: for backwards compat
        .bind(&email_hash)
        .bind(&email_encrypted)
        .bind(&contact_person_encrypted)
        .bind(&phone_encrypted)
        .bind(&address_encrypted)
        .bind(&license_number_encrypted)
        .fetch_one(&self.pool)
        .await?;

        // 🔒 DECRYPT on read - application-layer only
        let email = self.encryption.decrypt(&row.try_get::<String, _>("email_encrypted")?)
            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?;
        let contact_person = self.encryption.decrypt(&row.try_get::<String, _>("contact_person_encrypted")?)
            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?;
        let phone: Option<String> = row.try_get("phone_encrypted")?;
        let phone = match phone {
            Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
            None => None,
        };
        let address: Option<String> = row.try_get("address_encrypted")?;
        let address = match address {
            Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
            None => None,
        };
        let license_number: Option<String> = row.try_get("license_number_encrypted")?;
        let license_number = match license_number {
            Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
            None => None,
        };

        Ok(User {
            id: row.try_get("id")?,
            email,
            password_hash: row.try_get("password_hash")?,
            company_name: row.try_get("company_name")?,
            contact_person,
            phone,
            address,
            license_number,
            is_verified: row.try_get("is_verified")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        // 🔒 PRODUCTION: Query by hash, decrypt on read
        let email_hash = EncryptionService::hash_for_lookup(email);

        let row = query(
            r#"
            SELECT id, password_hash, company_name, is_verified, created_at, updated_at,
                   email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted
            FROM users
            WHERE email_hash = $1
            "#
        )
        .bind(&email_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                // Decrypt PII on read
                let email_encrypted: String = row.try_get("email_encrypted")?;
                let email = self.encryption.decrypt(&email_encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?;

                let contact_person_encrypted: String = row.try_get("contact_person_encrypted")?;
                let contact_person = self.encryption.decrypt(&contact_person_encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?;

                let phone: Option<String> = row.try_get("phone_encrypted")?;
                let phone = match phone {
                    Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
                    None => None,
                };

                let address: Option<String> = row.try_get("address_encrypted")?;
                let address = match address {
                    Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
                    None => None,
                };

                let license_number: Option<String> = row.try_get("license_number_encrypted")?;
                let license_number = match license_number {
                    Some(encrypted) => Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?),
                    None => None,
                };

                Ok(Some(User {
                    id: row.try_get("id")?,
                    email,
                    password_hash: row.try_get("password_hash")?,
                    company_name: row.try_get("company_name")?,
                    contact_person,
                    phone,
                    address,
                    license_number,
                    is_verified: row.try_get("is_verified")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                }))
            },
            None => Ok(None),
        }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = query(
            "SELECT id, email, password_hash, company_name, contact_person, phone, address, license_number, is_verified, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                password_hash: row.try_get("password_hash")?,
                company_name: row.try_get("company_name")?,
                contact_person: row.try_get("contact_person")?,
                phone: row.try_get("phone")?,
                address: row.try_get("address")?,
                license_number: row.try_get("license_number")?,
                is_verified: row.try_get("is_verified")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn update(&self, user_id: Uuid, request: &UpdateUserRequest) -> Result<User> {
        let mut query_str = "UPDATE users SET updated_at = $1".to_string();
        let mut param_count = 2;

        if let Some(ref company_name) = request.company_name {
            query_str.push_str(&format!(", company_name = ${}", param_count));
            param_count += 1;
        }

        if let Some(ref contact_person) = request.contact_person {
            query_str.push_str(&format!(", contact_person = ${}", param_count));
            param_count += 1;
        }

        if let Some(ref phone) = request.phone {
            query_str.push_str(&format!(", phone = ${}", param_count));
            param_count += 1;
        }

        if let Some(ref address) = request.address {
            query_str.push_str(&format!(", address = ${}", param_count));
            param_count += 1;
        }

        if let Some(ref license_number) = request.license_number {
            query_str.push_str(&format!(", license_number = ${}", param_count));
            param_count += 1;
        }

        query_str.push_str(&format!(" WHERE id = ${} RETURNING id, email, password_hash, company_name, contact_person, phone, address, license_number, is_verified, created_at, updated_at", param_count));

        let mut query_builder = query(&query_str)
            .bind(Utc::now());

        if let Some(ref company_name) = request.company_name {
            query_builder = query_builder.bind(company_name);
        }
        if let Some(ref contact_person) = request.contact_person {
            query_builder = query_builder.bind(contact_person);
        }
        if let Some(ref phone) = request.phone {
            query_builder = query_builder.bind(phone);
        }
        if let Some(ref address) = request.address {
            query_builder = query_builder.bind(address);
        }
        if let Some(ref license_number) = request.license_number {
            query_builder = query_builder.bind(license_number);
        }

        let row = query_builder
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(User {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            company_name: row.try_get("company_name")?,
            contact_person: row.try_get("contact_person")?,
            phone: row.try_get("phone")?,
            address: row.try_get("address")?,
            license_number: row.try_get("license_number")?,
            is_verified: row.try_get("is_verified")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn delete(&self, user_id: Uuid) -> Result<()> {
        let result = query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Resource not found".to_string()));
        }

        Ok(())
    }

    pub async fn email_exists(&self, email: &str) -> Result<bool> {
        let row = query("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) as exists")
            .bind(email)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get::<bool, _>("exists").unwrap_or(false))
    }
}