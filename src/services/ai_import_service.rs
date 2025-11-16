/// AI-powered inventory import orchestration service
/// Coordinates file parsing, AI analysis, validation, and batch import

use uuid::Uuid;
use sqlx::PgPool;
use crate::middleware::error_handling::{Result, AppError};
use crate::services::claude_ai_service::{ClaudeAIService, ClaudeRequestConfig, user_message};
use crate::services::file_parser_service::{FileParserService, ParsedFile};
use crate::models::ai_import::{
    AiImportSession, ColumnMapping, ImportStatus, MappedInventoryRow,
};

const ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert pharmaceutical inventory data analyst. Your task is to analyze supplier data files and map columns to a standardized pharmaceutical inventory schema.

IMPORTANT GUIDELINES:
1. NDC codes are the primary identifier - format: 5-4-2 digits (e.g., 12345-678-90)
2. Expiry dates can be in various formats: YYYY-MM-DD, MM/DD/YYYY, DD-MM-YYYY
3. Batch/Lot numbers are alphanumeric identifiers
4. Quantities should be integers
5. Prices should be numeric (dollars and cents)
6. Be conservative with confidence scores - only return high confidence when certain

Your response must be valid JSON with this exact structure:
{
  "mapping": {
    "ndc_code": "column_name_or_null",
    "brand_name": "column_name_or_null",
    "generic_name": "column_name_or_null",
    "manufacturer": "column_name_or_null",
    "quantity": "column_name_or_null",
    "batch_number": "column_name_or_null",
    "expiry_date": "column_name_or_null",
    "unit_price": "column_name_or_null",
    "storage_location": "column_name_or_null",
    "category": "column_name_or_null",
    "strength": "column_name_or_null",
    "dosage_form": "column_name_or_null"
  },
  "confidence_scores": {
    "ndc_code": 0.0-1.0,
    "brand_name": 0.0-1.0,
    ...
  },
  "warnings": [
    "Human-readable warning messages about data quality issues"
  ]
}"#;

pub struct AiImportService {
    db_pool: PgPool,
    claude_service: ClaudeAIService,
}

impl AiImportService {
    pub fn new(db_pool: PgPool, claude_api_key: String) -> Self {
        let claude_service = ClaudeAIService::new(claude_api_key, db_pool.clone());
        Self {
            db_pool,
            claude_service,
        }
    }

    /// Step 1: Create a new import session
    pub async fn create_session(
        &self,
        user_id: Uuid,
        filename: String,
        import_source: Option<String>,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO ai_import_sessions (
                id, user_id, original_filename, file_size_bytes,
                file_type, file_hash, status, import_source
            ) VALUES ($1, $2, $3, 0, 'pending', '', 'analyzing', $4)
            "#,
            session_id,
            user_id,
            filename,
            import_source
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!("Created AI import session: {} for user: {}", session_id, user_id);

        Ok(session_id)
    }

    /// Step 2: Analyze uploaded file with Claude AI
    pub async fn analyze_file(
        &self,
        session_id: Uuid,
        file_data: Vec<u8>,
        user_id: Uuid,
    ) -> Result<ColumnMapping> {
        // Get session
        let session = self.get_session(session_id).await?;

        // Check user quota
        if !self.claude_service.check_user_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly AI import limit reached. Please upgrade your plan.".to_string()
            ));
        }

        // Parse file
        tracing::info!("Parsing file for session: {}", session_id);
        let parsed_file = FileParserService::parse(&file_data, &session.original_filename)?;

        tracing::info!(
            "File parsed: {} rows, {} columns, type: {}",
            parsed_file.total_rows,
            parsed_file.headers.len(),
            parsed_file.file_type
        );

        // Update session with file metadata
        sqlx::query!(
            r#"
            UPDATE ai_import_sessions
            SET
                file_size_bytes = $1,
                file_type = $2,
                file_hash = $3,
                detected_format = $4,
                detected_columns = $5,
                total_rows = $6
            WHERE id = $7
            "#,
            file_data.len() as i64,
            parsed_file.file_type.to_string(),
            parsed_file.file_hash,
            parsed_file.file_type.to_string(),
            serde_json::to_value(&parsed_file.headers)?,
            parsed_file.total_rows as i32,
            session_id
        )
        .execute(&self.db_pool)
        .await?;

        // Prepare sample data for AI analysis (first 10 rows)
        let sample_rows: Vec<_> = parsed_file.rows.iter().take(10).collect();
        let sample_json = self.format_sample_for_ai(&parsed_file.headers, &sample_rows);

        // Build AI prompt
        let analysis_prompt = format!(
            r#"Analyze this pharmaceutical inventory data file and map columns to our schema.

FILE INFO:
- Format: {}
- Total Rows: {}
- Detected Columns: {}

COLUMN HEADERS:
{}

SAMPLE DATA (first 10 rows):
{}

TASK:
Map each detected column to the appropriate field in our pharmaceutical inventory schema.
Return JSON with mapping, confidence scores (0.0-1.0), and any data quality warnings.
"#,
            parsed_file.file_type,
            parsed_file.total_rows,
            parsed_file.headers.len(),
            parsed_file.headers.join(", "),
            sample_json
        );

        tracing::info!("Sending file analysis request to Claude AI");

        // Call Claude AI
        let config = ClaudeRequestConfig {
            max_tokens: 2048,
            temperature: Some(0.3), // Low temperature for consistency
            system_prompt: Some(ANALYSIS_SYSTEM_PROMPT.to_string()),
        };

        let ai_response = self.claude_service.send_message(
            vec![user_message(analysis_prompt)],
            config,
            user_id,
            Some(session_id),
        ).await?;

        tracing::info!(
            "AI analysis completed: {} tokens, ${:.4} cost",
            ai_response.input_tokens + ai_response.output_tokens,
            ai_response.cost_usd
        );

        // Parse AI response
        let analysis_result = self.parse_ai_mapping_response(&ai_response.content)?;

        // Update session with AI analysis results
        sqlx::query!(
            r#"
            UPDATE ai_import_sessions
            SET
                ai_mapping = $1,
                ai_confidence_scores = $2,
                ai_warnings = $3,
                ai_api_cost_usd = ai_api_cost_usd + $4,
                ai_tokens_used = ai_tokens_used + $5,
                analysis_completed_at = NOW(),
                status = 'mapping_review'
            WHERE id = $6
            "#,
            serde_json::to_value(&analysis_result.mapping)?,
            serde_json::to_value(&analysis_result.confidence_scores)?,
            &analysis_result.warnings.iter().map(|w| serde_json::json!(w)).collect::<Vec<_>>(),
            rust_decimal::Decimal::try_from(ai_response.cost_usd).unwrap_or_default(),
            (ai_response.input_tokens + ai_response.output_tokens) as i32,
            session_id
        )
        .execute(&self.db_pool)
        .await?;

        // Increment user usage
        self.claude_service.increment_user_usage(user_id, ai_response.cost_usd).await?;

        tracing::info!("AI analysis saved for session: {}", session_id);

        Ok(analysis_result.mapping)
    }

    /// Get session details
    pub async fn get_session(&self, session_id: Uuid) -> Result<AiImportSession> {
        let session = sqlx::query_as::<_, AiImportSession>(
            r#"SELECT * FROM ai_import_sessions WHERE id = $1"#
        )
        .bind(session_id)
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Import session not found".to_string()))?;

        Ok(session)
    }

    /// Format sample data for AI analysis
    fn format_sample_for_ai(&self, headers: &[String], rows: &[&Vec<String>]) -> String {
        let mut output = String::new();
        
        for (idx, row) in rows.iter().enumerate() {
            output.push_str(&format!("Row {}:\n", idx + 1));
            for (col_idx, header) in headers.iter().enumerate() {
                let value = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                output.push_str(&format!("  {}: {}\n", header, value));
            }
            output.push('\n');
        }

        output
    }

    /// Parse Claude's JSON response into structured mapping
    fn parse_ai_mapping_response(&self, content: &str) -> Result<AnalysisResult> {
        // Extract JSON from response (Claude sometimes adds text before/after)
        let json_start = content.find('{').ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("AI response missing JSON object"))
        })?;
        let json_end = content.rfind('}').ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("AI response missing JSON closing brace"))
        })?;

        let json_str = &content[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse AI JSON: {}", e)))?;

        let mapping: ColumnMapping = serde_json::from_value(
            parsed.get("mapping").cloned().unwrap_or(serde_json::json!({}))
        )?;

        let confidence_scores = parsed.get("confidence_scores").cloned().unwrap_or(serde_json::json!({}));

        let warnings: Vec<String> = parsed.get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(AnalysisResult {
            mapping,
            confidence_scores,
            warnings,
        })
    }
}

#[derive(Debug)]
struct AnalysisResult {
    mapping: ColumnMapping,
    confidence_scores: serde_json::Value,
    warnings: Vec<String>,
}
