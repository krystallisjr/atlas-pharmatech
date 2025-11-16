/// Inquiry Assistant Service - AI-powered negotiation and response suggestions
///
/// This service helps sellers respond professionally to buyer inquiries by:
/// - Analyzing inquiry context (product, pricing, history)
/// - Generating contextual response suggestions
/// - Providing negotiation strategies
/// - Maintaining conversation tone and professionalism

use crate::{
    middleware::error_handling::{Result, AppError},
    models::inquiry_assistant::*,
    services::claude_ai_service::{ClaudeAIService, ClaudeRequestConfig, user_message},
};
use sqlx::PgPool;
use uuid::Uuid;

const SYSTEM_PROMPT: &str = r#"You are a professional B2B pharmaceutical marketplace negotiation assistant.

Your role is to help sellers craft professional, strategic responses to buyer inquiries.

GUIDELINES:
1. Be professional and courteous at all times
2. Focus on win-win outcomes
3. Be transparent about product details (expiry, batch, etc.)
4. Suggest competitive but profitable pricing
5. Maintain compliance with pharmaceutical regulations
6. Build trust through clear communication

RESPONSE FORMAT:
Return ONLY a JSON object:
{
  "response": "The suggested message text",
  "reasoning": "Why this approach is effective",
  "negotiation_tips": ["Tip 1", "Tip 2"]
}

TONE GUIDELINES:
- Initial responses: Warm, welcoming, informative
- Negotiations: Collaborative, solution-oriented
- Pricing: Transparent, justified with value
- Closing: Confident, clear next steps
- Rejections: Respectful, leave door open for future

Remember: You're building long-term business relationships, not just closing single deals.
"#;

pub struct InquiryAssistantService {
    db_pool: PgPool,
    claude_service: ClaudeAIService,
}

impl InquiryAssistantService {
    pub fn new(db_pool: PgPool, claude_api_key: String) -> Self {
        let claude_service = ClaudeAIService::new(claude_api_key, db_pool.clone());
        Self {
            db_pool,
            claude_service,
        }
    }

    /// Generate AI suggestion for inquiry response
    pub async fn generate_suggestion(
        &self,
        inquiry_id: Uuid,
        user_id: Uuid,
        suggestion_type: SuggestionType,
        custom_instructions: Option<String>,
    ) -> Result<InquiryAiSuggestion> {
        // 1. Verify user owns this inquiry (as seller)
        let inquiry_ownership = sqlx::query!(
            r#"
            SELECT i.id, inv.user_id as seller_id
            FROM inquiries i
            JOIN inventory inv ON i.inventory_id = inv.id
            WHERE i.id = $1
            "#,
            inquiry_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquiry not found".to_string()))?;

        if inquiry_ownership.seller_id != user_id {
            return Err(AppError::Forbidden(
                "You don't have permission to access this inquiry".to_string()
            ));
        }

        // 2. Check quota
        if !self.claude_service.check_user_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly AI usage limit exceeded. Please upgrade your plan or wait for reset.".to_string()
            ));
        }

        // 3. Load inquiry context
        let context = self.load_inquiry_context(inquiry_id, user_id).await?;

        // 4. Load conversation history
        let conversation_history = self.load_conversation_history(inquiry_id).await?;

        // 5. Build prompt
        let prompt = self.build_prompt(
            &context,
            &conversation_history,
            &suggestion_type,
            custom_instructions.as_deref(),
        );

        // 6. Call Claude
        let config = ClaudeRequestConfig {
            max_tokens: 1024,
            temperature: Some(0.7), // Balanced for professional yet natural responses
            system_prompt: Some(SYSTEM_PROMPT.to_string()),
        };

        let suggestion_id = Uuid::new_v4();
        let claude_response = self.claude_service.send_message(
            vec![user_message(prompt)],
            config,
            user_id,
            Some(suggestion_id),
        ).await?;

        // 7. Parse AI response (strip markdown code fences if present)
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

        let ai_response: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| {
                tracing::error!("Failed to parse AI response: {}", e);
                tracing::error!("Raw response: {}", claude_response.content);
                tracing::error!("Cleaned content: {}", json_content);
                AppError::Internal(anyhow::anyhow!("AI returned invalid response"))
            })?;

        let response_text = ai_response["response"]
            .as_str()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing response field")))?
            .to_string();

        let reasoning = ai_response["reasoning"]
            .as_str()
            .map(|s| s.to_string());

        // 8. Save suggestion to database
        let suggestion = sqlx::query_as!(
            InquiryAiSuggestion,
            r#"
            INSERT INTO inquiry_ai_suggestions (
                id, inquiry_id, user_id, suggestion_type, suggestion_text,
                context_snapshot, ai_reasoning, ai_cost_usd, ai_tokens_used
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
            suggestion_id,
            inquiry_id,
            user_id,
            suggestion_type.to_string(),
            response_text,
            serde_json::to_value(&context).ok(),
            reasoning,
            rust_decimal::Decimal::try_from(claude_response.cost_usd).unwrap_or_default(),
            claude_response.input_tokens as i32 + claude_response.output_tokens as i32
        )
        .fetch_one(&self.db_pool)
        .await?;

        // 9. Increment usage quota
        sqlx::query!(
            r#"
            INSERT INTO user_ai_usage_limits (user_id, monthly_inquiry_assists_used)
            VALUES ($1, 1)
            ON CONFLICT (user_id)
            DO UPDATE SET
                monthly_inquiry_assists_used = user_ai_usage_limits.monthly_inquiry_assists_used + 1,
                updated_at = NOW()
            "#,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            "Inquiry suggestion generated: inquiry={}, type={}, user={}, cost=${}",
            inquiry_id,
            suggestion_type.to_string(),
            user_id,
            claude_response.cost_usd
        );

        Ok(suggestion)
    }

    /// Load comprehensive inquiry context for AI
    async fn load_inquiry_context(
        &self,
        inquiry_id: Uuid,
        user_id: Uuid,
    ) -> Result<InquiryContext> {
        let context = sqlx::query!(
            r#"
            SELECT
                p.brand_name || ' ' || p.generic_name || ' ' || p.strength as product_name,
                i_inq.quantity_requested,
                inv.quantity as quantity_available,
                inv.unit_price,
                inv.batch_number,
                inv.expiry_date,
                buyer.company_name as buyer_company,
                seller.company_name as seller_company,
                i_inq.status as inquiry_status,
                (SELECT COUNT(*) FROM inquiry_messages WHERE inquiry_id = i_inq.id) as message_count
            FROM inquiries i_inq
            JOIN inventory inv ON i_inq.inventory_id = inv.id
            JOIN pharmaceuticals p ON inv.pharmaceutical_id = p.id
            JOIN users buyer ON i_inq.buyer_id = buyer.id
            JOIN users seller ON inv.user_id = seller.id
            WHERE i_inq.id = $1 AND inv.user_id = $2
            "#,
            inquiry_id,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquiry context not found".to_string()))?;

        Ok(InquiryContext {
            product_name: Some(context.product_name.unwrap_or_default()),
            quantity_requested: context.quantity_requested,
            quantity_available: context.quantity_available,
            unit_price: context.unit_price.map(|p| p.to_string().parse().unwrap_or(0.0)),
            batch_number: Some(context.batch_number), // Wrap in Some as it's non-null from query
            expiry_date: Some(context.expiry_date.to_string()), // Convert NaiveDate to String
            buyer_company: context.buyer_company,
            seller_company: context.seller_company,
            message_count: context.message_count.unwrap_or(0) as i32,
            inquiry_status: context.inquiry_status.unwrap_or_else(|| "unknown".to_string()),
        })
    }

    /// Load conversation history
    async fn load_conversation_history(&self, inquiry_id: Uuid) -> Result<ConversationHistory> {
        let messages = sqlx::query!(
            r#"
            SELECT
                msg.message as text,
                msg.created_at as timestamp,
                CASE
                    WHEN msg.sender_id = i.buyer_id THEN 'buyer'
                    ELSE 'seller'
                END as sender
            FROM inquiry_messages msg
            JOIN inquiries i ON msg.inquiry_id = i.id
            WHERE msg.inquiry_id = $1
            ORDER BY msg.created_at ASC
            "#,
            inquiry_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        let conversation_messages = messages.into_iter()
            .map(|m| ConversationMessage {
                sender: m.sender.unwrap_or_default(),
                text: m.text,
                timestamp: m.timestamp.unwrap_or_else(chrono::Utc::now), // Use current time as fallback
            })
            .collect();

        Ok(ConversationHistory {
            messages: conversation_messages,
        })
    }

    /// Build AI prompt with full context
    fn build_prompt(
        &self,
        context: &InquiryContext,
        history: &ConversationHistory,
        suggestion_type: &SuggestionType,
        custom_instructions: Option<&str>,
    ) -> String {
        let mut prompt = format!(
            r#"INQUIRY CONTEXT:

Product: {}
Buyer Company: {}
Seller Company: {}
Quantity Requested: {}
Quantity Available: {}
Unit Price: ${}
Batch Number: {}
Expiry Date: {}
Inquiry Status: {}
Message Count: {}

"#,
            context.product_name.as_ref().unwrap_or(&"Unknown".to_string()),
            context.buyer_company,
            context.seller_company,
            context.quantity_requested,
            context.quantity_available,
            context.unit_price.unwrap_or(0.0),
            context.batch_number.as_ref().unwrap_or(&"N/A".to_string()),
            context.expiry_date.as_ref().unwrap_or(&"N/A".to_string()),
            context.inquiry_status,
            context.message_count
        );

        // Add conversation history
        if !history.messages.is_empty() {
            prompt.push_str("CONVERSATION HISTORY:\n");
            for msg in &history.messages {
                prompt.push_str(&format!(
                    "{}: {}\n",
                    msg.sender.to_uppercase(),
                    msg.text
                ));
            }
            prompt.push_str("\n");
        }

        // Add task-specific instructions
        prompt.push_str(&format!("TASK: Generate a {} response.\n\n",
            match suggestion_type {
                SuggestionType::InitialResponse => "welcoming initial",
                SuggestionType::Negotiation => "collaborative negotiation",
                SuggestionType::PricingAdjustment => "pricing adjustment",
                SuggestionType::TermsClarification => "terms clarification",
                SuggestionType::ClosingDeal => "deal closing",
                SuggestionType::FollowUp => "follow-up",
                SuggestionType::Rejection => "polite rejection",
            }
        ));

        // Add type-specific context
        match suggestion_type {
            SuggestionType::InitialResponse => {
                prompt.push_str("This is the first response to a new inquiry. Welcome the buyer, confirm product availability, and provide key details.\n");
            }
            SuggestionType::Negotiation => {
                prompt.push_str("The buyer is negotiating. Find a middle ground that's profitable but fair. Consider bulk discounts, payment terms, or delivery options.\n");
            }
            SuggestionType::PricingAdjustment => {
                prompt.push_str("Propose a price adjustment. Justify the price with market conditions, product quality, or volume discounts.\n");
            }
            SuggestionType::TermsClarification => {
                prompt.push_str("Clarify business terms like payment, delivery, quality guarantees, and compliance documentation.\n");
            }
            SuggestionType::ClosingDeal => {
                prompt.push_str("The deal is almost done. Summarize the agreement, confirm details, and provide clear next steps.\n");
            }
            SuggestionType::FollowUp => {
                prompt.push_str("Follow up on a previous message. Be courteous and check on their decision without being pushy.\n");
            }
            SuggestionType::Rejection => {
                prompt.push_str("Politely decline this inquiry. Explain the reason (if appropriate) and leave the door open for future opportunities.\n");
            }
        }

        // Add custom instructions if provided
        if let Some(instructions) = custom_instructions {
            prompt.push_str(&format!("\nADDITIONAL INSTRUCTIONS: {}\n", instructions));
        }

        prompt.push_str("\nGenerate a professional response that the seller can use (possibly with minor edits).");

        prompt
    }

    /// Accept suggestion and send as message
    pub async fn accept_suggestion(
        &self,
        suggestion_id: Uuid,
        user_id: Uuid,
        edited_text: Option<String>,
    ) -> Result<Uuid> {
        // 1. Load suggestion
        let suggestion = sqlx::query_as!(
            InquiryAiSuggestion,
            "SELECT * FROM inquiry_ai_suggestions WHERE id = $1",
            suggestion_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Suggestion not found".to_string()))?;

        // 2. Verify ownership
        if suggestion.user_id != user_id {
            return Err(AppError::Forbidden("Not your suggestion".to_string()));
        }

        // 3. Determine final text (edited or original)
        let final_text = edited_text.as_ref()
            .unwrap_or(&suggestion.suggestion_text);
        let was_edited = edited_text.is_some();

        // 4. Create inquiry message
        let message_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO inquiry_messages (id, inquiry_id, sender_id, message)
            VALUES ($1, $2, $3, $4)
            "#,
            message_id,
            suggestion.inquiry_id,
            user_id,
            final_text
        )
        .execute(&self.db_pool)
        .await?;

        // 5. Update suggestion as accepted
        sqlx::query!(
            r#"
            UPDATE inquiry_ai_suggestions
            SET was_accepted = TRUE,
                was_edited = $1,
                final_message_id = $2
            WHERE id = $3
            "#,
            was_edited,
            message_id,
            suggestion_id
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            "Suggestion accepted: id={}, inquiry={}, edited={}",
            suggestion_id,
            suggestion.inquiry_id,
            was_edited
        );

        Ok(message_id)
    }

    /// Get suggestion by ID
    pub async fn get_suggestion(&self, suggestion_id: Uuid, user_id: Uuid) -> Result<InquiryAiSuggestion> {
        let suggestion = sqlx::query_as!(
            InquiryAiSuggestion,
            "SELECT * FROM inquiry_ai_suggestions WHERE id = $1 AND user_id = $2",
            suggestion_id,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Suggestion not found".to_string()))?;

        Ok(suggestion)
    }

    /// Get all suggestions for an inquiry
    pub async fn get_inquiry_suggestions(
        &self,
        inquiry_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<InquiryAiSuggestion>> {
        let suggestions = sqlx::query_as!(
            InquiryAiSuggestion,
            r#"
            SELECT s.*
            FROM inquiry_ai_suggestions s
            WHERE s.inquiry_id = $1 AND s.user_id = $2
            ORDER BY s.created_at DESC
            "#,
            inquiry_id,
            user_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(suggestions)
    }

    /// Get user's quota status
    pub async fn get_quota_status(&self, user_id: Uuid) -> Result<(i32, i32, i32)> {
        let quota = sqlx::query!(
            r#"
            SELECT
                monthly_inquiry_assist_limit,
                monthly_inquiry_assists_used,
                monthly_inquiry_assist_limit - monthly_inquiry_assists_used as remaining
            FROM user_ai_usage_limits
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match quota {
            Some(q) => Ok((
                q.monthly_inquiry_assist_limit,
                q.monthly_inquiry_assists_used,
                q.remaining.unwrap_or(200) // Calculated field should always exist, but handle gracefully
            )),
            None => Ok((200, 0, 200)), // Default quota
        }
    }
}
