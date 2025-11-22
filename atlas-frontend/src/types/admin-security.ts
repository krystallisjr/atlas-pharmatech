// ============================================================================
// Admin Security Types - Type definitions for security monitoring features
// ============================================================================
//
// These types match the backend Rust types exactly from:
// - src/handlers/admin_security.rs
// - src/services/api_quota_service.rs
// - src/services/encryption_key_rotation_service.rs
//
// ============================================================================

// ============================================================================
// API Usage Types
// ============================================================================

export interface ApiUsageFilters {
  user_id?: string;
  endpoint?: string;
  start_date?: string;  // ISO 8601 datetime
  end_date?: string;    // ISO 8601 datetime
  limit?: number;
  offset?: number;
}

export interface ApiUsageRecord {
  id: string;
  user_id: string;
  user_email: string | null;
  endpoint: string;
  tokens_input: number;
  tokens_output: number;
  total_tokens: number;
  cost_cents: number;
  latency_ms: number;
  created_at: string;  // ISO 8601 datetime
}

export interface EndpointUsage {
  endpoint: string;
  request_count: number;
  total_cost_cents: number;
  avg_latency_ms: number;
}

export interface UserUsage {
  user_id: string;
  user_email: string;
  request_count: number;
  total_cost_cents: number;
  quota_tier: QuotaTier;
}

export interface TimeSeriesPoint {
  date: string;
  requests: number;
  cost_cents: number;
}

export interface ApiUsageAnalytics {
  total_requests: number;
  total_cost_cents: number;
  total_tokens: number;
  avg_latency_ms: number;
  usage_by_endpoint: EndpointUsage[];
  usage_by_user: UserUsage[];
  usage_over_time: TimeSeriesPoint[];
  recent_requests: ApiUsageRecord[];
}

// ============================================================================
// Quota Management Types
// ============================================================================

export type QuotaTier = 'Free' | 'Basic' | 'Pro' | 'Enterprise';

export interface UserQuotaInfo {
  user_id: string;
  user_email: string;
  quota_tier: QuotaTier;
  monthly_limit: number | null;  // null = unlimited
  monthly_used: number;
  monthly_remaining: number | null;
  usage_percent: number;
  total_cost_cents: number;
  is_over_quota: boolean;
}

export interface QuotaUpdateRequest {
  quota_tier: QuotaTier;
}

// ============================================================================
// Encryption Key Rotation Types
// ============================================================================

export interface EncryptionKeyInfo {
  id: string;
  key_version: number;
  status: string;  // "Active", "Deprecated", "Rotated"
  is_active: boolean;
  created_at: string;  // ISO 8601 datetime
  valid_until: string; // ISO 8601 datetime
  age_days: number;
  days_until_expiry: number;
}

export interface KeyRotationEvent {
  id: string;
  old_version: number;
  new_version: number;
  rotated_at: string;  // ISO 8601 datetime
  rotated_by_email: string | null;
  rotation_reason: string | null;
}

export type RotationStatus = 'OK' | 'SOON' | 'OVERDUE';

export interface EncryptionStatus {
  active_key: EncryptionKeyInfo;
  rotation_status: RotationStatus;
  days_until_rotation: number;
  all_keys: EncryptionKeyInfo[];
  rotation_history: KeyRotationEvent[];
}

export interface KeyRotationRequest {
  reason?: string;
}

// ============================================================================
// Metrics Types
// ============================================================================

export interface MetricsSummary {
  http_requests_total: number;
  http_requests_per_minute: number;
  avg_request_duration_ms: number;
  active_connections: number;
  auth_failures_total: number;
  auth_failures_last_hour: number;
  db_pool_active: number;
  db_pool_idle: number;
  request_duration_p50: number;
  request_duration_p95: number;
  request_duration_p99: number;
  status_code_breakdown: Record<string, number>;
}

// ============================================================================
// Rate Limiting Types
// ============================================================================

export interface RateLimitEntry {
  ip_address: string;
  current_tokens: number;
  max_tokens: number;
  last_request: string;  // ISO 8601 datetime
}

export interface IpLimitInfo {
  ip_address: string;
  hit_count: number;
  last_hit: string;  // ISO 8601 datetime
}

export interface RateLimitConfig {
  auth_limit: string;      // e.g., "5 requests per 15 minutes"
  api_limit: string;        // e.g., "100 requests per minute"
  public_limit: string;     // e.g., "20 requests per 15 minutes"
}

export interface RateLimitStatus {
  active_rate_limits: RateLimitEntry[];
  top_limited_ips: IpLimitInfo[];
  configuration: RateLimitConfig;
}
