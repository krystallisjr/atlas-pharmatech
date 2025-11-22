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

    /// Get reference to the database pool (for direct queries)
    pub fn pool(&self) -> &PgPool {
        &self.pool
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
            RETURNING id, password_hash, company_name, is_verified, role, created_at, updated_at,
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
            role: row.try_get("role")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        // 🔒 PRODUCTION: Query by hash, decrypt on read
        let email_hash = EncryptionService::hash_for_lookup(email);

        let row = query(
            r#"
            SELECT id, email, email_hash, password_hash, company_name, is_verified, role, created_at, updated_at,
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
                // Decrypt PII on read (handle NULL for migration compatibility)
                let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
                    if let Some(encrypted) = email_encrypted {
                        self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
                    } else {
                        // Fallback to plaintext email if encrypted field is NULL (migration compatibility)
                        row.try_get("email")?
                    }
                } else {
                    row.try_get("email")?  // Fallback to plaintext
                };

                let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
                    if let Some(encrypted) = contact_person_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
                    if let Some(encrypted) = phone_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
                    if let Some(encrypted) = address_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
                    if let Some(encrypted) = license_number_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(Some(User {
                    id: row.try_get("id")?,
                    email,
                    password_hash: row.try_get("password_hash")?,
                    company_name: row.try_get("company_name")?,
                    contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
                    phone,
                    address,
                    license_number,
                    is_verified: row.try_get("is_verified")?,
                    role: row.try_get("role")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                }))
            },
            None => Ok(None),
        }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        // 🔒 PRODUCTION: Query encrypted columns, decrypt on read
        let row = query(
            r#"
            SELECT id, email, email_hash, password_hash, company_name, is_verified, role, created_at, updated_at,
                   email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted
            FROM users
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                // Decrypt PII on read (handle NULL for migration compatibility)
                let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
                    if let Some(encrypted) = email_encrypted {
                        self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
                    } else {
                        // Fallback to plaintext email if encrypted field is NULL (migration compatibility)
                        row.try_get("email")?
                    }
                } else {
                    row.try_get("email")?  // Fallback to plaintext
                };

                let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
                    if let Some(encrypted) = contact_person_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
                    if let Some(encrypted) = phone_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
                    if let Some(encrypted) = address_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
                    if let Some(encrypted) = license_number_encrypted {
                        Some(self.encryption.decrypt(&encrypted)
                            .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(Some(User {
                    id: row.try_get("id")?,
                    email,
                    password_hash: row.try_get("password_hash")?,
                    company_name: row.try_get("company_name")?,
                    contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
                    phone,
                    address,
                    license_number,
                    is_verified: row.try_get("is_verified")?,
                    role: row.try_get("role")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                }))
            },
            None => Ok(None),
        }
    }

    pub async fn update(&self, user_id: Uuid, request: &UpdateUserRequest) -> Result<User> {
        // 🔒 SECURITY: Use individual UPDATE statements instead of dynamic query building
        // This prevents SQL injection risks from query concatenation and makes queries easier to audit

        let now = Utc::now();

        // Always update timestamp
        query("UPDATE users SET updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Update company_name if provided
        if let Some(ref company_name) = request.company_name {
            query("UPDATE users SET company_name = $1, updated_at = $2 WHERE id = $3")
                .bind(company_name)
                .bind(now)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Update contact_person if provided (with encryption)
        if let Some(ref contact_person) = request.contact_person {
            let encrypted = self.encryption.encrypt(contact_person)?;
            query("UPDATE users SET contact_person_encrypted = $1, updated_at = $2 WHERE id = $3")
                .bind(&encrypted)
                .bind(now)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Update phone if provided (with encryption)
        if let Some(ref phone) = request.phone {
            let encrypted = self.encryption.encrypt(phone)?;
            query("UPDATE users SET phone_encrypted = $1, updated_at = $2 WHERE id = $3")
                .bind(&encrypted)
                .bind(now)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Update address if provided (with encryption)
        if let Some(ref address) = request.address {
            let encrypted = self.encryption.encrypt(address)?;
            query("UPDATE users SET address_encrypted = $1, updated_at = $2 WHERE id = $3")
                .bind(&encrypted)
                .bind(now)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Update license_number if provided (with encryption)
        if let Some(ref license_number) = request.license_number {
            let encrypted = self.encryption.encrypt(license_number)?;
            query("UPDATE users SET license_number_encrypted = $1, updated_at = $2 WHERE id = $3")
                .bind(&encrypted)
                .bind(now)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Fetch and return updated user
        self.find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found after update".to_string()))
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

    // ========================================================================
    // ADMIN METHODS
    // ========================================================================

    /// List all users with pagination and optional filters
    /// 🔒 PRODUCTION: List users with encrypted PII decryption (admin only)
    pub async fn list_users(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        role_filter: Option<crate::models::user::UserRole>,
        verified_filter: Option<bool>,
        search_query: Option<String>,
    ) -> Result<Vec<User>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        // Query encrypted columns
        let mut query_str = r#"
            SELECT id, email, email_hash, password_hash, company_name, is_verified, role,
                   created_at, updated_at,
                   email_encrypted, contact_person_encrypted, phone_encrypted,
                   address_encrypted, license_number_encrypted
            FROM users
            WHERE 1=1
        "#.to_string();

        let mut param_count = 1;

        if role_filter.is_some() {
            query_str.push_str(&format!(" AND role = ${}", param_count));
            param_count += 1;
        }

        if verified_filter.is_some() {
            query_str.push_str(&format!(" AND is_verified = ${}", param_count));
            param_count += 1;
        }

        if search_query.is_some() {
            query_str.push_str(&format!(" AND company_name ILIKE ${}", param_count));
            param_count += 1;
        }

        query_str.push_str(" ORDER BY created_at DESC");
        query_str.push_str(&format!(" LIMIT ${} OFFSET ${}", param_count, param_count + 1));

        let mut query_builder = query(&query_str);

        if let Some(role) = role_filter {
            query_builder = query_builder.bind(role);
        }
        if let Some(verified) = verified_filter {
            query_builder = query_builder.bind(verified);
        }
        if let Some(search) = search_query {
            query_builder = query_builder.bind(format!("%{}%", search));
        }

        let rows = query_builder
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        // Decrypt PII for each user (production-safe error handling)
        let mut users = Vec::new();
        for row in rows {
            // Decrypt email
            let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
                if let Some(encrypted) = email_encrypted {
                    self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
                } else {
                    row.try_get("email")?
                }
            } else {
                row.try_get("email")?
            };

            // Decrypt contact_person
            let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
                if let Some(encrypted) = contact_person_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt phone
            let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
                if let Some(encrypted) = phone_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt address
            let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
                if let Some(encrypted) = address_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt license_number
            let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
                if let Some(encrypted) = license_number_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            users.push(User {
                id: row.try_get("id")?,
                email,
                password_hash: row.try_get("password_hash")?,
                company_name: row.try_get("company_name")?,
                contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
                phone,
                address,
                license_number,
                is_verified: row.try_get("is_verified")?,
                role: row.try_get("role")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }

    /// Count total users with optional filters
    pub async fn count_users(
        &self,
        role_filter: Option<crate::models::user::UserRole>,
        verified_filter: Option<bool>,
    ) -> Result<i64> {
        let mut query_str = "SELECT COUNT(*) as count FROM users WHERE 1=1".to_string();
        let mut param_count = 1;

        if role_filter.is_some() {
            query_str.push_str(&format!(" AND role = ${}", param_count));
            param_count += 1;
        }

        if verified_filter.is_some() {
            query_str.push_str(&format!(" AND is_verified = ${}", param_count));
        }

        let mut query_builder = query(&query_str);

        if let Some(role) = role_filter {
            query_builder = query_builder.bind(role);
        }
        if let Some(verified) = verified_filter {
            query_builder = query_builder.bind(verified);
        }

        let row = query_builder.fetch_one(&self.pool).await?;
        Ok(row.try_get::<i64, _>("count")?)
    }

    /// 🔒 PRODUCTION: Set user verification status (admin only) - with PII decryption
    pub async fn set_verified(&self, user_id: Uuid, verified: bool) -> Result<User> {
        let row = query(
            r#"
            UPDATE users
            SET is_verified = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, email, email_hash, password_hash, company_name, is_verified, role,
                      created_at, updated_at,
                      email_encrypted, contact_person_encrypted, phone_encrypted,
                      address_encrypted, license_number_encrypted
            "#
        )
        .bind(verified)
        .bind(Utc::now())
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Decrypt PII
        let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
            if let Some(encrypted) = email_encrypted {
                self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
            } else {
                row.try_get("email")?
            }
        } else {
            row.try_get("email")?
        };

        let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
            if let Some(encrypted) = contact_person_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
            if let Some(encrypted) = phone_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
            if let Some(encrypted) = address_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
            if let Some(encrypted) = license_number_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(User {
            id: row.try_get("id")?,
            email,
            password_hash: row.try_get("password_hash")?,
            company_name: row.try_get("company_name")?,
            contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
            phone,
            address,
            license_number,
            is_verified: row.try_get("is_verified")?,
            role: row.try_get("role")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// 🔒 PRODUCTION: Set user role (superadmin only) - with PII decryption
    pub async fn set_role(
        &self,
        user_id: Uuid,
        role: crate::models::user::UserRole,
        changed_by: Uuid,
    ) -> Result<User> {
        let row = query(
            r#"
            UPDATE users
            SET role = $1, role_changed_at = $2, role_changed_by = $3, updated_at = $4
            WHERE id = $5
            RETURNING id, email, email_hash, password_hash, company_name, is_verified, role,
                      created_at, updated_at,
                      email_encrypted, contact_person_encrypted, phone_encrypted,
                      address_encrypted, license_number_encrypted
            "#
        )
        .bind(role)
        .bind(Utc::now())
        .bind(changed_by)
        .bind(Utc::now())
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Decrypt PII
        let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
            if let Some(encrypted) = email_encrypted {
                self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
            } else {
                row.try_get("email")?
            }
        } else {
            row.try_get("email")?
        };

        let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
            if let Some(encrypted) = contact_person_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
            if let Some(encrypted) = phone_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
            if let Some(encrypted) = address_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
            if let Some(encrypted) = license_number_encrypted {
                Some(self.encryption.decrypt(&encrypted)
                    .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(User {
            id: row.try_get("id")?,
            email,
            password_hash: row.try_get("password_hash")?,
            company_name: row.try_get("company_name")?,
            contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
            phone,
            address,
            license_number,
            is_verified: row.try_get("is_verified")?,
            role: row.try_get("role")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// 🔒 PRODUCTION: Get pending verification queue (unverified users only) - with PII decryption
    pub async fn get_verification_queue(&self) -> Result<Vec<User>> {
        let rows = query(
            r#"
            SELECT id, email, email_hash, password_hash, company_name, is_verified, role,
                   created_at, updated_at,
                   email_encrypted, contact_person_encrypted, phone_encrypted,
                   address_encrypted, license_number_encrypted
            FROM users
            WHERE is_verified = false AND role = 'user'
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        // Decrypt PII for each user (production-safe error handling)
        let mut users = Vec::new();
        for row in rows {
            // Decrypt email
            let email = if let Ok(email_encrypted) = row.try_get::<Option<String>, _>("email_encrypted") {
                if let Some(encrypted) = email_encrypted {
                    self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?
                } else {
                    row.try_get("email")?
                }
            } else {
                row.try_get("email")?
            };

            // Decrypt contact_person
            let contact_person = if let Ok(contact_person_encrypted) = row.try_get::<Option<String>, _>("contact_person_encrypted") {
                if let Some(encrypted) = contact_person_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt phone
            let phone = if let Ok(phone_encrypted) = row.try_get::<Option<String>, _>("phone_encrypted") {
                if let Some(encrypted) = phone_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt address
            let address = if let Ok(address_encrypted) = row.try_get::<Option<String>, _>("address_encrypted") {
                if let Some(encrypted) = address_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            // Decrypt license_number
            let license_number = if let Ok(license_number_encrypted) = row.try_get::<Option<String>, _>("license_number_encrypted") {
                if let Some(encrypted) = license_number_encrypted {
                    Some(self.encryption.decrypt(&encrypted)
                        .map_err(|e| AppError::Internal(anyhow!("Decryption failed: {:?}", e)))?)
                } else {
                    None
                }
            } else {
                None
            };

            users.push(User {
                id: row.try_get("id")?,
                email,
                password_hash: row.try_get("password_hash")?,
                company_name: row.try_get("company_name")?,
                contact_person: contact_person.unwrap_or_else(|| "Unknown".to_string()),
                phone,
                address,
                license_number,
                is_verified: row.try_get("is_verified")?,
                role: row.try_get("role")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }
}