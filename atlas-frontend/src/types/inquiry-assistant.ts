// Inquiry Assistant Types

export type SuggestionType =
  | 'initial_response'
  | 'negotiation'
  | 'pricing_adjustment'
  | 'terms_clarification'
  | 'closing_deal'
  | 'follow_up'
  | 'rejection';

export interface GenerateSuggestionRequest {
  suggestion_type: SuggestionType;
  custom_instructions?: string;
}

export interface InquirySuggestion {
  id: string;
  inquiry_id: string;
  suggestion_type: string;
  suggestion_text: string;
  reasoning: string | null;
  context_used: InquiryContext;
  ai_cost_usd: string;
  created_at: string;
}

export interface InquiryContext {
  product_name: string | null;
  quantity_requested: number;
  quantity_available: number;
  unit_price: number | null;
  batch_number: string | null;
  expiry_date: string | null;
  buyer_company: string;
  seller_company: string;
  message_count: number;
  inquiry_status: string;
}

export interface AcceptSuggestionRequest {
  edited_text?: string;
}

export interface AcceptSuggestionResponse {
  message_id: string;
  was_edited: boolean;
}

export interface AssistantQuotaStatus {
  assist_limit: number;
  assists_used: number;
  assists_remaining: number;
}
