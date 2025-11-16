// Natural Language Query Types

export interface NlQueryRequest {
  query: string;
}

export interface QueryResponse {
  id: string;
  status: 'processing' | 'analyzing' | 'success' | 'failed' | 'invalid_sql' | 'quota_exceeded';
  query_text: string;
  response_type: 'sql_query' | 'conversation';
  generated_sql: string | null;
  execution_time_ms: number | null;
  result_count: number | null;
  results: any[] | null;
  explanation: string | null;
  ai_response: string | null; // For conversational responses
  ai_cost_usd: string;
  error_message: string | null;
  created_at: string;
}

export interface QueryHistoryItem {
  id: string;
  query_text: string;
  status: string;
  result_count: number | null;
  execution_time_ms: number | null;
  created_at: string;
}

export interface SaveFavoriteRequest {
  query_text: string;
  description?: string;
  category?: string;
}

export interface FavoriteQuery {
  id: string;
  query_text: string;
  description: string | null;
  category: string | null;
  created_at: string;
}

export interface QuotaStatus {
  query_limit: number;
  queries_used: number;
  queries_remaining: number;
}
