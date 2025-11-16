/// Natural Language Query Service - AI-powered SQL generation with security
///
/// This service allows users to query their data using natural language.
/// Claude AI converts natural language to SQL, which is then validated and executed safely.

use crate::{
    middleware::error_handling::{Result, AppError},
    models::nl_query::*,
    services::claude_ai_service::{ClaudeAIService, ClaudeRequestConfig, user_message},
};
use sqlx::{PgPool, Row, Column};
use uuid::Uuid;
use std::time::Instant;

const MAX_RESULTS: i64 = 100;

// Database schema for AI context
const DATABASE_SCHEMA: &str = r#"
# Atlas Pharma Database Schema

## Tables

### pharmaceuticals
Columns: id (UUID), brand_name (TEXT), generic_name (TEXT), ndc_code (TEXT), manufacturer (TEXT),
         category (TEXT), strength (TEXT), dosage_form (TEXT), storage_requirements (TEXT)
Indexes: brand_name, generic_name, ndc_code, manufacturer, category

### inventory
Columns: id (UUID), user_id (UUID), pharmaceutical_id (UUID), batch_number (TEXT),
         quantity (INTEGER), expiry_date (DATE), unit_price (DECIMAL), storage_location (TEXT),
         status (TEXT: 'available', 'reserved', 'sold', 'expired')
Indexes: user_id, pharmaceutical_id, expiry_date, status
Note: ALWAYS filter by user_id for security

### inquiries
Columns: id (UUID), inventory_id (UUID), buyer_id (UUID), quantity_requested (INTEGER),
         message (TEXT), status (TEXT: 'pending', 'negotiating', 'accepted', 'rejected', 'converted_to_transaction'),
         last_message_at (TIMESTAMPTZ)
Indexes: inventory_id, buyer_id, status

### transactions
Columns: id (UUID), inquiry_id (UUID), seller_id (UUID), buyer_id (UUID),
         quantity (INTEGER), unit_price (DECIMAL), total_price (DECIMAL),
         transaction_date (TIMESTAMPTZ), status (TEXT: 'pending', 'completed', 'cancelled')
Note: seller_id or buyer_id must match user_id

## Relationships
- inventory.pharmaceutical_id → pharmaceuticals.id
- inventory.user_id → users.id (REQUIRED FILTER)
- inquiries.inventory_id → inventory.id
- inquiries.buyer_id → users.id
- transactions.seller_id → users.id
- transactions.buyer_id → users.id
"#;

const SYSTEM_PROMPT: &str = r#"You are an expert AI business consultant specializing in pharmaceutical B2B operations. Think of yourself as a knowledgeable partner who deeply understands the pharmaceutical industry, supply chain dynamics, regulatory landscape, and market trends.

YOUR EXPERTISE:
- Pharmaceutical industry operations, compliance, and best practices
- Supply chain management and inventory optimization
- Pricing strategies, market analysis, and competitive positioning
- Business development, negotiation tactics, and relationship building
- Quality control, storage requirements, and regulatory compliance (FDA, WHO-GMP)
- Market trends, demand forecasting, and risk management

YOUR PERSONALITY:
- Conversational and approachable, not robotic
- Proactive with suggestions and insights
- Honest and transparent about limitations
- Strategic thinker who sees the bigger picture
- Detail-oriented when it matters

RESPONSE MODES:

🗣️ CONVERSATION MODE (for advice, strategy, explanations, analysis)
Use when the user wants to:
- Understand industry concepts or regulations
- Get strategic business advice
- Learn about best practices
- Discuss pricing, negotiation, or market trends
- Analyze situations or make decisions
- Get recommendations or suggestions

Be conversational, insightful, and helpful. Provide specific, actionable advice. Use examples. Think critically. Challenge assumptions when helpful. Offer alternatives.

Response format:
{
  "type": "conversation",
  "answer": "Natural, helpful response with specific insights and recommendations"
}

📊 SQL QUERY MODE (for specific data retrieval)
Use when the user needs to:
- View their inventory, transactions, or inquiries
- Find marketplace listings or suppliers
- Get specific numbers, counts, or analytics
- Search for products or companies
- Check statuses or dates

CRITICAL SQL SECURITY:
1. User's data: WHERE user_id = 'USER_ID_PLACEHOLDER'
2. Marketplace/competitors: WHERE user_id != 'USER_ID_PLACEHOLDER' OR omit user filter
3. ONLY SELECT (never INSERT, UPDATE, DELETE, DROP, ALTER)
4. Always LIMIT 100

Response format:
{
  "type": "sql_query",
  "sql": "SELECT ... FROM ... WHERE ... LIMIT 100",
  "explanation": "What this shows"
}

DATABASE SCHEMA:
- inventory: user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status ('available', 'reserved', 'sold', 'expired')
- pharmaceuticals: id, brand_name, generic_name, ndc_code, manufacturer, category, strength, dosage_form, storage_requirements
- users: id, company_name, company_type, is_verified
- inquiries: id, inventory_id, buyer_id, quantity_requested, message, status
- transactions: id, seller_id, buyer_id, quantity, unit_price, total_price, transaction_date, status

EXAMPLE CONVERSATIONS:

Q: "Should I accept this offer for 500 units of Ibuprofen at $0.80 per unit?"
A: {
  "type": "conversation",
  "answer": "Let's break this down. At $0.80/unit for Ibuprofen, you need to consider:\n\n1. **Your cost basis** - What did you pay? Aim for at least 25-35% margin\n2. **Market rate** - Generic ibuprofen typically trades at $0.85-$1.20 in B2B, so $0.80 is below market\n3. **Opportunity cost** - Could you sell it higher elsewhere? Check marketplace listings\n4. **Expiry date** - If it's expiring soon, take the deal. If shelf life is good, negotiate higher\n5. **Batch size** - 500 units is medium volume. You could counter at $0.95-$1.00\n\nMy recommendation: If expiry is >6 months away, counter at $0.95. If 3-6 months, $0.85 is fair. If <3 months, accept $0.80 to move it quickly.\n\nWant me to check current marketplace prices for comparison?"
}

Q: "How do I handle expired inventory?"
A: {
  "type": "conversation",
  "answer": "Expired pharmaceutical inventory requires careful handling due to regulatory compliance:\n\n**Immediate Actions:**\n1. Segregate immediately - separate from saleable stock to prevent accidental sale\n2. Mark clearly as 'EXPIRED - DO NOT USE'\n3. Update your system status to 'expired'\n\n**Disposal Options:**\n- Reverse distribution: Return to manufacturer/distributor (may get partial credit)\n- Licensed disposal companies: Must be DEA-authorized for controlled substances\n- Take-back programs: Some manufacturers offer these\n\n**Prevention Strategies:**\n- Implement FEFO (First Expired, First Out) rotation\n- Set alerts for items expiring in 90 days\n- Offer early-expiry discounts (30-40% off at 6 months)\n- Share with charitable organizations if regulations allow\n\n**Documentation:**\nKeep records for 3+ years: what expired, disposal method, certificates of destruction.\n\nWant me to show you what's expiring soon in your inventory?"
}

Q: "Show me items expiring in 60 days"
A: {
  "type": "sql_query",
  "sql": "SELECT p.brand_name, p.generic_name, i.batch_number, i.quantity, i.expiry_date, i.unit_price, CAST(i.quantity * i.unit_price AS DECIMAL(10,2)) as total_value FROM inventory i JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id WHERE i.user_id = 'USER_ID_PLACEHOLDER' AND i.expiry_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '60 days' AND i.status = 'available' ORDER BY i.expiry_date ASC LIMIT 100",
  "explanation": "Your inventory expiring in the next 60 days with total value calculations"
}

GUIDELINES:
- Be natural and conversational, not stiff or formal
- Give specific, actionable advice with numbers and examples
- When appropriate, suggest follow-up queries or actions
- Admit when you don't have enough context and ask clarifying questions
- Think strategically - consider multiple angles and trade-offs
- Be proactive - if you notice something important, mention it
- Use industry terminology correctly but explain complex concepts
- Balance optimism with realism

Replace USER_ID_PLACEHOLDER with actual user_id in SQL queries.
"#;

pub struct NlQueryService {
    db_pool: PgPool,
    claude_service: ClaudeAIService,
}

impl NlQueryService {
    pub fn new(db_pool: PgPool, claude_api_key: String) -> Self {
        let claude_service = ClaudeAIService::new(claude_api_key, db_pool.clone());
        Self {
            db_pool,
            claude_service,
        }
    }

    /// Execute a natural language query
    pub async fn execute_query(
        &self,
        user_id: Uuid,
        query_text: String,
    ) -> Result<NlQuerySession> {
        let start_time = Instant::now();

        // 1. Create session record
        let session_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO nl_query_sessions (id, user_id, query_text, status)
             VALUES ($1, $2, $3, 'processing')",
            session_id,
            user_id,
            query_text
        )
        .execute(&self.db_pool)
        .await?;

        // 2. Check quota (using existing Claude service quota check)
        if !self.claude_service.check_user_quota(user_id).await? {
            self.mark_session_failed(
                session_id,
                "Quota exceeded: Monthly AI usage limit reached".to_string(),
                "quota_exceeded"
            ).await?;
            return Err(AppError::QuotaExceeded(
                "Monthly AI usage limit exceeded. Please upgrade your plan or wait for reset.".to_string()
            ));
        }

        // 3. Generate SQL with Claude
        let prompt = format!(
            "{}\n\nUSER_ID: {}\n\nQUESTION: {}",
            DATABASE_SCHEMA,
            user_id,
            query_text
        );

        let config = ClaudeRequestConfig {
            max_tokens: 2048,
            temperature: Some(0.3), // Lower temperature for more consistent SQL generation
            system_prompt: Some(SYSTEM_PROMPT.to_string()),
        };

        let claude_response = match self.claude_service.send_message(
            vec![user_message(prompt)],
            config,
            user_id,
            Some(session_id),
        ).await {
            Ok(response) => response,
            Err(e) => {
                self.mark_session_failed(
                    session_id,
                    format!("AI error: {}", e),
                    "failed"
                ).await?;
                return Err(e);
            }
        };

        // 4. Parse AI response (strip markdown code fences if present)
        let content = claude_response.content.trim();
        let json_content = if content.starts_with("```json") {
            content.trim_start_matches("```json")
                   .trim_start_matches("```")
                   .trim_end_matches("```")
                   .trim()
        } else if content.starts_with("```") {
            content.trim_start_matches("```")
                   .trim_end_matches("```")
                   .trim()
        } else {
            content
        };

        let ai_response: AiAssistantResponse = match serde_json::from_str(json_content) {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Failed to parse AI response: {}", e);
                tracing::error!("Raw response: {}", claude_response.content);
                tracing::error!("Cleaned content: {}", json_content);
                self.mark_session_failed(
                    session_id,
                    format!("Failed to parse AI response: {}", e),
                    "failed"
                ).await?;
                return Err(AppError::Internal(anyhow::anyhow!("AI returned invalid response")));
            }
        };

        // 5. Handle response based on type
        match ai_response {
            AiAssistantResponse::Conversation { answer, .. } => {
                // Conversational response - no SQL execution needed
                let total_time_ms = start_time.elapsed().as_millis() as i64;

                sqlx::query!(
                    r#"
                    UPDATE nl_query_sessions
                    SET result_data = $1,
                        execution_time_ms = $2,
                        ai_cost_usd = $3,
                        ai_tokens_used = $4,
                        status = 'success'
                    WHERE id = $5
                    "#,
                    serde_json::json!({"answer": answer}),
                    total_time_ms as i32,
                    rust_decimal::Decimal::try_from(claude_response.cost_usd).unwrap_or_default(),
                    (claude_response.input_tokens + claude_response.output_tokens) as i32,
                    session_id
                )
                .execute(&self.db_pool)
                .await?;

                // Update usage quota
                self.update_nl_query_usage(user_id).await?;

                tracing::info!(
                    "NL Conversation completed: user={}, query={}, time={}ms, cost=${:.6}",
                    user_id, query_text, total_time_ms, claude_response.cost_usd
                );

                return self.get_session(session_id).await;
            }
            AiAssistantResponse::SqlQuery { sql, explanation, .. } => {
                // SQL query response - execute the query
                // Save generated SQL
                sqlx::query!(
                    "UPDATE nl_query_sessions SET generated_sql = $1 WHERE id = $2",
                    sql,
                    session_id
                )
                .execute(&self.db_pool)
                .await?;

                // Validate and sanitize SQL
                let validated_sql = match self.validate_and_inject_user_filter(&sql, user_id) {
                    Ok(sql) => sql,
                    Err(e) => {
                        self.mark_session_failed(
                            session_id,
                            format!("SQL validation failed: {}", e),
                            "invalid_sql"
                        ).await?;
                        return Err(e);
                    }
                };

                // Execute the query
                let query_start = Instant::now();
                let result_rows = match sqlx::query(&validated_sql)
                    .fetch_all(&self.db_pool)
                    .await {
                    Ok(rows) => rows,
                    Err(e) => {
                        self.mark_session_failed(
                            session_id,
                            format!("Query execution failed: {}", e),
                            "failed"
                        ).await?;
                        return Err(AppError::Database(e));
                    }
                };
                let execution_time_ms = query_start.elapsed().as_millis() as i32;

                // Get result count and convert rows to JSON
                let result_count = result_rows.len() as i32;

                // Convert PgRow to JSON objects
                let mut json_rows = Vec::new();
                for row in result_rows {
                    let mut json_row = serde_json::Map::new();
                    for column in row.columns() {
                        let value = row.try_get_raw(column.ordinal())
                            .ok()
                            .and_then(|raw| {
                                // Try to decode as different types
                                if let Ok(v) = row.try_get::<String, _>(column.ordinal()) {
                                    Some(serde_json::Value::String(v))
                                } else if let Ok(v) = row.try_get::<i32, _>(column.ordinal()) {
                                    Some(serde_json::Value::Number(v.into()))
                                } else if let Ok(v) = row.try_get::<i64, _>(column.ordinal()) {
                                    Some(serde_json::Value::Number(v.into()))
                                } else if let Ok(v) = row.try_get::<f64, _>(column.ordinal()) {
                                    serde_json::Number::from_f64(v).map(serde_json::Value::Number)
                                } else if let Ok(v) = row.try_get::<bool, _>(column.ordinal()) {
                                    Some(serde_json::Value::Bool(v))
                                } else {
                                    Some(serde_json::Value::Null)
                                }
                            })
                            .unwrap_or(serde_json::Value::Null);

                        json_row.insert(column.name().to_string(), value);
                    }
                    json_rows.push(serde_json::Value::Object(json_row));
                }

                // Store the actual results for display
                let result_data = serde_json::Value::Array(json_rows);
                let total_time_ms = start_time.elapsed().as_millis() as i64;

                // Update session with results
                sqlx::query!(
                    r#"
                    UPDATE nl_query_sessions
                    SET validated_sql = $1,
                        result_data = $2,
                        result_count = $3,
                        execution_time_ms = $4,
                        ai_cost_usd = $5,
                        ai_tokens_used = $6,
                        status = 'success'
                    WHERE id = $7
                    "#,
                    validated_sql,
                    serde_json::json!(result_data),
                    result_count,
                    execution_time_ms,
                    rust_decimal::Decimal::try_from(claude_response.cost_usd).unwrap_or_default(),
                    claude_response.input_tokens as i32 + claude_response.output_tokens as i32,
                    session_id
                )
                .execute(&self.db_pool)
                .await?;

                // Update usage quota
                self.update_nl_query_usage(user_id).await?;

                tracing::info!(
                    "NL Query executed successfully: user={}, query={}, results={}, time={}ms, cost=${}",
                    user_id,
                    query_text,
                    result_count,
                    total_time_ms,
                    claude_response.cost_usd
                );

                // Return session
                self.get_session(session_id).await
            }
        }
    }

    /// Validate SQL and inject user_id filter for security
    fn validate_and_inject_user_filter(&self, sql: &str, user_id: Uuid) -> Result<String> {
        let sql_upper = sql.to_uppercase();

        // 1. Block dangerous operations
        let dangerous_keywords = vec![
            "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE",
            "CREATE", "GRANT", "REVOKE", "EXEC", "EXECUTE"
        ];

        for keyword in dangerous_keywords {
            if sql_upper.contains(keyword) {
                return Err(AppError::BadRequest(
                    format!("Forbidden SQL operation: {}", keyword)
                ));
            }
        }

        // 2. Ensure it's a SELECT query
        if !sql_upper.trim().starts_with("SELECT") {
            return Err(AppError::BadRequest(
                "Only SELECT queries are allowed".to_string()
            ));
        }

        // 3. Check for LIMIT clause (should already be there, but enforce)
        let sql_with_limit = if !sql_upper.contains("LIMIT") {
            format!("{} LIMIT {}", sql.trim_end_matches(';'), MAX_RESULTS)
        } else {
            sql.to_string()
        };

        // 4. Verify user_id is in the query (AI should have added it)
        // This is a sanity check - if AI didn't add user filtering, block the query
        let has_user_filter = sql_upper.contains("USER_ID")
            || sql_upper.contains("SELLER_ID")
            || sql_upper.contains("BUYER_ID");

        if !has_user_filter {
            tracing::warn!("Query missing user_id filter, blocking: {}", sql);
            return Err(AppError::BadRequest(
                "Query must filter by user_id for security".to_string()
            ));
        }

        // 5. Replace $1 placeholder with actual user_id (parameterized query)
        let final_sql = sql_with_limit.replace("$1", &format!("'{}'", user_id));

        Ok(final_sql)
    }

    /// Mark session as failed
    async fn mark_session_failed(
        &self,
        session_id: Uuid,
        error_message: String,
        status: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE nl_query_sessions SET status = $1, error_message = $2 WHERE id = $3",
            status,
            error_message,
            session_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    /// Update NL query usage quota
    async fn update_nl_query_usage(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_ai_usage_limits (user_id, monthly_nl_queries_used)
            VALUES ($1, 1)
            ON CONFLICT (user_id)
            DO UPDATE SET
                monthly_nl_queries_used = user_ai_usage_limits.monthly_nl_queries_used + 1,
                updated_at = NOW()
            "#,
            user_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: Uuid) -> Result<NlQuerySession> {
        let session = sqlx::query_as!(
            NlQuerySession,
            "SELECT * FROM nl_query_sessions WHERE id = $1",
            session_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Query session not found".to_string()))?;

        Ok(session)
    }

    /// Get user's query history
    pub async fn get_history(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<NlQuerySession>> {
        let sessions = sqlx::query_as!(
            NlQuerySession,
            r#"
            SELECT * FROM nl_query_sessions
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(sessions)
    }

    /// Save query as favorite
    pub async fn save_favorite(
        &self,
        user_id: Uuid,
        query_text: String,
        description: Option<String>,
        category: Option<String>,
    ) -> Result<NlQueryFavorite> {
        let favorite = sqlx::query_as!(
            NlQueryFavorite,
            r#"
            INSERT INTO nl_query_favorites (user_id, query_text, description, category)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, query_text) DO UPDATE
            SET description = EXCLUDED.description, category = EXCLUDED.category
            RETURNING *
            "#,
            user_id,
            query_text,
            description,
            category
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(favorite)
    }

    /// Get user's favorites
    pub async fn get_favorites(&self, user_id: Uuid) -> Result<Vec<NlQueryFavorite>> {
        let favorites = sqlx::query_as!(
            NlQueryFavorite,
            "SELECT * FROM nl_query_favorites WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(favorites)
    }

    /// Get user's quota status
    pub async fn get_quota_status(&self, user_id: Uuid) -> Result<(i32, i32, i32)> {
        let quota = sqlx::query!(
            r#"
            SELECT
                monthly_nl_query_limit,
                monthly_nl_queries_used,
                monthly_nl_query_limit - monthly_nl_queries_used as remaining
            FROM user_ai_usage_limits
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match quota {
            Some(q) => Ok((
                q.monthly_nl_query_limit,
                q.monthly_nl_queries_used,
                q.remaining.unwrap_or(100) // Calculated field should always exist, but handle gracefully
            )),
            None => Ok((100, 0, 100)), // Default quota
        }
    }
}
