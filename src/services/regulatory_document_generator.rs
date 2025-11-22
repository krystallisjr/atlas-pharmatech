// 📋 REGULATORY DOCUMENT GENERATOR - PRODUCTION RAG SYSTEM
// Generates Certificate of Analysis (CoA), GDP, and GMP documents using Claude AI + RAG
// Follows exact patterns from existing services - PRODUCTION READY

use crate::middleware::error_handling::{Result, AppError};
use crate::services::{
    ClaudeAIService, ClaudeEmbeddingService, ClaudeMessage, ClaudeRequestConfig,
    Ed25519SignatureService, KnowledgeEntry,
};
use anyhow::anyhow;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Document generation request
#[derive(Debug, Deserialize)]
pub struct GenerateDocumentRequest {
    pub document_type: DocumentType,
    pub product_name: Option<String>,
    pub batch_number: Option<String>,
    pub manufacturer: Option<String>,
    pub test_results: Option<serde_json::Value>,
    pub custom_fields: Option<serde_json::Value>,
}

/// Document type
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum DocumentType {
    CoA,  // Certificate of Analysis
    GDP,  // Good Distribution Practice
    GMP,  // Good Manufacturing Practice
}

impl DocumentType {
    fn as_str(&self) -> &str {
        match self {
            DocumentType::CoA => "CoA",
            DocumentType::GDP => "GDP",
            DocumentType::GMP => "GMP",
        }
    }
}

/// Generated document response
#[derive(Debug, Serialize)]
pub struct GeneratedDocument {
    pub id: Uuid,
    pub document_type: String,
    pub document_number: String,
    pub title: String,
    pub content: serde_json::Value,
    pub content_hash: String,
    #[serde(rename = "generated_signature")]
    pub signature: String,
    pub public_key: String,
    pub rag_context: Vec<RagContextEntry>,
    pub status: String,
    pub generated_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// RAG context entry (knowledge base chunks used)
#[derive(Debug, Serialize, Clone)]
pub struct RagContextEntry {
    pub id: Uuid,
    pub section_title: String,
    pub regulation_source: Option<String>,
    pub similarity: f64,
}

/// Regulatory Document Generator Service
///
/// This service generates regulatory documents using:
/// 1. Semantic search (RAG) to retrieve relevant regulations
/// 2. Claude AI to generate compliant content
/// 3. Ed25519 signatures for non-repudiation
/// 4. Immutable audit ledger with blockchain-style chain hashing
pub struct RegulatoryDocumentGenerator {
    db_pool: PgPool,
    claude_service: ClaudeAIService,
    embedding_service: ClaudeEmbeddingService,
    signature_service: Ed25519SignatureService,
}

impl RegulatoryDocumentGenerator {
    /// Create new regulatory document generator
    pub fn new(
        db_pool: PgPool,
        api_key: String,
        encryption_key: &str,
        system_user_id: Uuid,
    ) -> Result<Self> {
        let claude_service = ClaudeAIService::new(api_key.clone(), db_pool.clone());
        let embedding_service =
            ClaudeEmbeddingService::new(db_pool.clone(), api_key, system_user_id)?;
        let signature_service = Ed25519SignatureService::new(db_pool.clone(), encryption_key)?;

        Ok(Self {
            db_pool,
            claude_service,
            embedding_service,
            signature_service,
        })
    }

    /// Generate a regulatory document with RAG + Claude AI + Ed25519 signature
    ///
    /// # Flow:
    /// 1. Retrieve relevant regulations using semantic search (RAG)
    /// 2. Build prompt with RAG context
    /// 3. Generate document content using Claude AI
    /// 4. Sign document with user's Ed25519 private key
    /// 5. Store in database with immutable audit trail
    ///
    /// # Arguments
    /// * `request` - Document generation request
    /// * `user_id` - User generating the document
    ///
    /// # Returns
    /// * `Ok(GeneratedDocument)` - Generated and signed document
    pub async fn generate_document(
        &self,
        request: GenerateDocumentRequest,
        user_id: Uuid,
    ) -> Result<GeneratedDocument> {
        tracing::info!(
            "Generating {} document for user {}",
            request.document_type.as_str(),
            user_id
        );

        // Step 1: Ensure user has Ed25519 keypair
        if !self.signature_service.has_keypair(user_id).await? {
            self.signature_service
                .generate_user_keypair(user_id)
                .await?;
            tracing::info!("Generated Ed25519 keypair for user {}", user_id);
        }

        // Step 2: Retrieve relevant regulations using RAG (semantic search)
        let rag_context = self
            .retrieve_rag_context(&request.document_type, &request)
            .await?;

        tracing::info!(
            "Retrieved {} relevant regulations for RAG context",
            rag_context.len()
        );

        // Step 3: Generate document content using Claude AI + RAG
        let content = self
            .generate_document_content(&request, &rag_context, user_id)
            .await?;

        // Step 4: Generate document number
        let document_number = self
            .generate_document_number(&request.document_type)
            .await?;

        // Step 5: Calculate content hash (SHA-256)
        let content_json = serde_json::to_string(&content)?;
        let content_hash = Sha256::digest(content_json.as_bytes());
        let content_hash_hex = hex::encode(&content_hash);

        // Step 6: Sign document with user's Ed25519 private key
        let (signature, _) = self
            .signature_service
            .sign_document(user_id, &content_json)
            .await?;

        // Step 7: Get user's public key for verification
        let public_key = self
            .signature_service
            .get_user_public_key(user_id)
            .await?
            .ok_or_else(|| anyhow!("User has no public key"))?;

        // Step 8: Store document in database
        let document_id = self
            .store_document(
                &request.document_type,
                &document_number,
                &content,
                &content_hash_hex,
                &signature,
                &rag_context,
                user_id,
            )
            .await?;

        // Step 9: Create immutable audit ledger entry
        self.create_ledger_entry(
            document_id,
            "generated",
            &content_hash_hex,
            &signature,
            &public_key,
        )
        .await?;

        tracing::info!(
            "Successfully generated {} document: {} (id: {})",
            request.document_type.as_str(),
            document_number,
            document_id
        );

        // Generate title from document type and number
        let title = format!("{} - {}", request.document_type.as_str(), document_number);

        Ok(GeneratedDocument {
            id: document_id,
            document_type: request.document_type.as_str().to_string(),
            document_number,
            title,
            content,
            content_hash: content_hash_hex,
            signature,
            public_key,
            rag_context: rag_context
                .iter()
                .map(|entry| RagContextEntry {
                    id: entry.id,
                    section_title: entry.section_title.clone(),
                    regulation_source: entry.regulation_source.clone(),
                    similarity: entry.similarity,
                })
                .collect(),
            status: "draft".to_string(),
            generated_by: user_id.to_string(),
            created_at: chrono::Utc::now(),
        })
    }

    /// Approve document (adds approval signature)
    pub async fn approve_document(
        &self,
        document_id: Uuid,
        approver_user_id: Uuid,
    ) -> Result<()> {
        // Retrieve document
        let doc = sqlx::query!(
            "SELECT content, content_hash FROM regulatory_documents WHERE id = $1",
            document_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        let content_json = serde_json::to_string(&doc.content)?;

        // Sign with approver's key
        let (approval_signature, _) = self
            .signature_service
            .sign_document(approver_user_id, &content_json)
            .await?;

        // Update document
        sqlx::query!(
            "UPDATE regulatory_documents SET approved_signature = $1, status = 'approved' WHERE id = $2",
            approval_signature,
            document_id
        )
        .execute(&self.db_pool)
        .await?;

        // Get approver's public key
        let public_key = self
            .signature_service
            .get_user_public_key(approver_user_id)
            .await?
            .ok_or_else(|| anyhow!("Approver has no public key"))?;

        // Create ledger entry
        self.create_ledger_entry(
            document_id,
            "approved",
            &doc.content_hash,
            &approval_signature,
            &public_key,
        )
        .await?;

        tracing::info!("Document {} approved by user {}", document_id, approver_user_id);

        Ok(())
    }

    /// Verify document signature and ledger integrity
    pub async fn verify_document(&self, document_id: Uuid) -> Result<bool> {
        // Verify document signature
        let doc = sqlx::query!(
            r#"
            SELECT
                rd.content,
                rd.content_hash,
                rd.generated_signature,
                rd.generated_by,
                u.ed25519_public_key
            FROM regulatory_documents rd
            JOIN users u ON rd.generated_by = u.id
            WHERE rd.id = $1
            "#,
            document_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        let signature = doc
            .generated_signature
            .ok_or_else(|| anyhow!("Document has no signature"))?;
        let public_key = doc
            .ed25519_public_key
            .ok_or_else(|| anyhow!("Document generator has no public key"))?;

        let signature_valid = self
            .signature_service
            .verify_signature(&doc.content_hash, &signature, &public_key)?;

        if !signature_valid {
            return Ok(false);
        }

        // Verify ledger chain integrity
        let chain_valid = self
            .signature_service
            .verify_ledger_chain_integrity(document_id)
            .await?;

        Ok(chain_valid)
    }

    // ============================================================================
    // PRIVATE HELPER METHODS
    // ============================================================================

    /// Retrieve relevant regulations using RAG (semantic search)
    async fn retrieve_rag_context(
        &self,
        document_type: &DocumentType,
        request: &GenerateDocumentRequest,
    ) -> Result<Vec<KnowledgeEntry>> {
        // Check if knowledge base has any entries
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM regulatory_knowledge_base WHERE document_type = $1"
        )
        .bind(document_type.as_str())
        .fetch_one(&self.db_pool)
        .await?;

        // Skip semantic search if knowledge base is empty
        if count == 0 {
            tracing::warn!(
                "Knowledge base is empty for {}, skipping RAG retrieval",
                document_type.as_str()
            );
            return Ok(Vec::new());
        }

        // Build search query based on document type and request
        let search_query = self.build_rag_search_query(document_type, request);

        // Perform semantic search
        let entries = self
            .embedding_service
            .semantic_search(&search_query, Some(document_type.as_str()), 10)
            .await?;

        Ok(entries)
    }

    /// Build search query for RAG
    fn build_rag_search_query(
        &self,
        document_type: &DocumentType,
        request: &GenerateDocumentRequest,
    ) -> String {
        match document_type {
            DocumentType::CoA => {
                format!(
                    "Certificate of Analysis requirements for pharmaceutical product {} batch {}",
                    request.product_name.as_deref().unwrap_or("pharmaceutical product"),
                    request.batch_number.as_deref().unwrap_or("batch")
                )
            }
            DocumentType::GDP => {
                format!(
                    "Good Distribution Practice guidelines for {} distribution and storage",
                    request.product_name.as_deref().unwrap_or("pharmaceutical")
                )
            }
            DocumentType::GMP => {
                format!(
                    "Good Manufacturing Practice requirements for {} manufacturing by {}",
                    request.product_name.as_deref().unwrap_or("pharmaceutical"),
                    request.manufacturer.as_deref().unwrap_or("manufacturer")
                )
            }
        }
    }

    /// Generate document content using Claude AI with RAG context
    async fn generate_document_content(
        &self,
        request: &GenerateDocumentRequest,
        rag_context: &[KnowledgeEntry],
        user_id: Uuid,
    ) -> Result<serde_json::Value> {
        // Build prompt with RAG context
        let prompt = self.build_generation_prompt(request, rag_context);

        let messages = vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let config = ClaudeRequestConfig {
            max_tokens: 4096,
            temperature: Some(0.3), // Low temperature for consistency
            system_prompt: Some(self.get_document_generation_system_prompt(&request.document_type)),
        };

        // Call Claude API
        let response = self
            .claude_service
            .send_message(messages, config, user_id, None)
            .await?;

        // Strip markdown code fences if present
        let json_str = Self::strip_markdown_fences(&response.content);

        // Parse JSON response
        let content: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            tracing::error!("Failed to parse document JSON from Claude: {}", e);
            tracing::debug!("Claude response: {}", response.content);
            tracing::debug!("After stripping fences: {}", json_str);
            AppError::Internal(anyhow!("Failed to parse document content: {}", e))
        })?;

        Ok(content)
    }

    /// Strip markdown code fences from Claude's response
    fn strip_markdown_fences(text: &str) -> String {
        let trimmed = text.trim();

        // Check for ```json ... ``` or ``` ... ```
        if trimmed.starts_with("```") {
            let without_start = trimmed.strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .unwrap_or(trimmed);

            let without_end = without_start.strip_suffix("```")
                .unwrap_or(without_start);

            without_end.trim().to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Build generation prompt with RAG context
    fn build_generation_prompt(
        &self,
        request: &GenerateDocumentRequest,
        rag_context: &[KnowledgeEntry],
    ) -> String {
        let mut prompt = format!(
            "Generate a compliant {} document based on the following information and regulatory context.\n\n",
            request.document_type.as_str()
        );

        // Add request details
        prompt.push_str("## Document Details\n");
        if let Some(product) = &request.product_name {
            prompt.push_str(&format!("Product: {}\n", product));
        }
        if let Some(batch) = &request.batch_number {
            prompt.push_str(&format!("Batch Number: {}\n", batch));
        }
        if let Some(manufacturer) = &request.manufacturer {
            prompt.push_str(&format!("Manufacturer: {}\n", manufacturer));
        }
        if let Some(test_results) = &request.test_results {
            // Safe: serde_json::to_string_pretty only fails for types with custom serialization that return errors
            // Since we're using serde_json::Value (already validated JSON), this cannot fail
            let test_results_str = serde_json::to_string_pretty(test_results)
                .unwrap_or_else(|_| test_results.to_string());
            prompt.push_str(&format!("\nTest Results:\n{}\n", test_results_str));
        }
        if let Some(custom) = &request.custom_fields {
            let custom_str = serde_json::to_string_pretty(custom)
                .unwrap_or_else(|_| custom.to_string());
            prompt.push_str(&format!("\nAdditional Information:\n{}\n", custom_str));
        }

        // Add RAG context (relevant regulations)
        prompt.push_str("\n## Relevant Regulatory Requirements\n");
        for (i, entry) in rag_context.iter().enumerate() {
            prompt.push_str(&format!(
                "\n### Regulation {} - {} (Similarity: {:.2})\n",
                i + 1,
                entry.section_title,
                entry.similarity
            ));
            if let Some(source) = &entry.regulation_source {
                prompt.push_str(&format!("Source: {}\n", source));
            }
            if let Some(section) = &entry.regulation_section {
                prompt.push_str(&format!("Section: {}\n", section));
            }
            prompt.push_str(&format!("\n{}\n", &entry.content[..entry.content.len().min(500)]));
        }

        prompt.push_str("\n\nGenerate the document in valid JSON format following the template structure. Ensure all regulatory requirements are addressed.");

        prompt
    }

    /// Get system prompt for document generation
    fn get_document_generation_system_prompt(&self, document_type: &DocumentType) -> String {
        match document_type {
            DocumentType::CoA => {
                "You are an expert pharmaceutical quality control specialist. Generate a Certificate of Analysis (CoA) \
                that complies with FDA 21 CFR Part 211 and ICH Q6A guidelines. The CoA must include:\n\
                - Product identification and batch information\n\
                - Test methods and specifications\n\
                - Test results with acceptance criteria\n\
                - Conclusion statement (Pass/Fail)\n\
                - Signature blocks\n\n\
                Output valid JSON with the structure: {\"header\": {...}, \"tests\": [...], \"conclusion\": \"...\"}".to_string()
            }
            DocumentType::GDP => {
                "You are an expert in pharmaceutical distribution and supply chain compliance. Generate a Good Distribution Practice (GDP) \
                document that complies with EU GDP Guidelines 2013/C 68/01. The document must address:\n\
                - Quality management system\n\
                - Storage and transportation conditions\n\
                - Temperature monitoring and validation\n\
                - Personnel qualifications\n\
                - Documentation and record keeping\n\n\
                Output valid JSON with the structure: {\"header\": {...}, \"sections\": [...], \"certifications\": [...]}".to_string()
            }
            DocumentType::GMP => {
                "You are an expert in pharmaceutical manufacturing and GMP compliance. Generate a Good Manufacturing Practice (GMP) \
                document that complies with FDA 21 CFR Part 211 and ICH Q7. The document must cover:\n\
                - Manufacturing facility qualifications\n\
                - Process validation\n\
                - Equipment calibration and maintenance\n\
                - Personnel training\n\
                - Quality control procedures\n\n\
                Output valid JSON with the structure: {\"header\": {...}, \"processes\": [...], \"validations\": [...]}".to_string()
            }
        }
    }

    /// Generate unique document number
    async fn generate_document_number(&self, document_type: &DocumentType) -> Result<String> {
        let doc_type_prefix = document_type.as_str();
        let year = chrono::Utc::now().date_naive().year();

        // Count existing documents of this type this year to get next sequence number
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM regulatory_documents
             WHERE document_type = $1
             AND EXTRACT(YEAR FROM created_at) = $2"
        )
        .bind(doc_type_prefix)
        .bind(year as i32)
        .fetch_one(&self.db_pool)
        .await?;

        let seq_num = count + 1;
        let document_number = format!("{}-{}-{:06}", doc_type_prefix, year, seq_num);

        Ok(document_number)
    }

    /// Store document in database
    async fn store_document(
        &self,
        document_type: &DocumentType,
        document_number: &str,
        content: &serde_json::Value,
        content_hash: &str,
        signature: &str,
        rag_context: &[KnowledgeEntry],
        generated_by: Uuid,
    ) -> Result<Uuid> {
        // Build RAG context JSON
        let rag_context_json = serde_json::json!({
            "entries": rag_context.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "title": e.section_title,
                    "source": e.regulation_source,
                    "similarity": e.similarity
                })
            }).collect::<Vec<_>>()
        });

        // Generate title from document type and number
        let title = format!("{} - {}", document_type.as_str(), document_number);

        let doc = sqlx::query!(
            r#"
            INSERT INTO regulatory_documents
                (document_type, document_number, title, content, content_hash, generated_signature, rag_context, status, generated_by)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, 'draft', $8)
            RETURNING id
            "#,
            document_type.as_str(),
            document_number,
            title,
            content,
            content_hash,
            signature,
            rag_context_json,
            generated_by
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(doc.id)
    }

    /// Create immutable audit ledger entry
    async fn create_ledger_entry(
        &self,
        document_id: Uuid,
        operation: &str,
        content_hash: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO regulatory_document_ledger
                (document_id, operation, content_hash, signature, signature_public_key)
            VALUES
                ($1, $2, $3, $4, $5)
            "#,
            document_id,
            operation,
            content_hash,
            signature,
            public_key
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
