// ============================================================================
// Admin Security Service - API client for security monitoring features
// ============================================================================
//
// Provides methods to interact with backend security monitoring endpoints:
// - API usage analytics and tracking
// - User quota management
// - Encryption key rotation
// - System metrics monitoring
// - Rate limiting status
//
// All endpoints require admin or superadmin role (enforced by backend middleware)
//
// ============================================================================

import { apiClient } from '../api-client';
import type {
  ApiUsageFilters,
  ApiUsageAnalytics,
  UserQuotaInfo,
  QuotaUpdateRequest,
  EncryptionStatus,
  KeyRotationRequest,
  EncryptionKeyInfo,
  MetricsSummary,
  RateLimitStatus,
} from '@/types/admin-security';

export class AdminSecurityService {
  // ==========================================================================
  // API Usage Analytics
  // ==========================================================================

  /**
   * GET /api/admin/security/api-usage
   *
   * Fetches API usage analytics with optional filters
   *
   * @param filters - Optional filters for date range, user, endpoint, pagination
   * @returns Comprehensive usage analytics including charts data, top users, recent requests
   *
   * @example
   * const analytics = await AdminSecurityService.getApiUsageAnalytics({
   *   start_date: '2024-01-01T00:00:00Z',
   *   end_date: '2024-01-31T23:59:59Z',
   *   limit: 50
   * });
   */
  static async getApiUsageAnalytics(filters?: ApiUsageFilters): Promise<ApiUsageAnalytics> {
    const queryParams = new URLSearchParams();

    if (filters?.user_id) queryParams.append('user_id', filters.user_id);
    if (filters?.endpoint) queryParams.append('endpoint', filters.endpoint);
    if (filters?.start_date) queryParams.append('start_date', filters.start_date);
    if (filters?.end_date) queryParams.append('end_date', filters.end_date);
    if (filters?.limit !== undefined) queryParams.append('limit', filters.limit.toString());
    if (filters?.offset !== undefined) queryParams.append('offset', filters.offset.toString());

    const url = `/api/admin/security/api-usage${queryParams.toString() ? `?${queryParams.toString()}` : ''}`;
    return apiClient.get<ApiUsageAnalytics>(url);
  }

  // ==========================================================================
  // Quota Management
  // ==========================================================================

  /**
   * GET /api/admin/security/quotas
   *
   * Fetches all users' quota information including usage statistics
   *
   * @returns Array of user quota info with usage percentages and limits
   *
   * @example
   * const quotas = await AdminSecurityService.getUserQuotas();
   * const overQuotaUsers = quotas.filter(q => q.is_over_quota);
   */
  static async getUserQuotas(): Promise<UserQuotaInfo[]> {
    return apiClient.get<UserQuotaInfo[]>('/api/admin/security/quotas');
  }

  /**
   * PUT /api/admin/security/quotas/:user_id
   *
   * Updates a user's quota tier (Superadmin only)
   *
   * @param userId - UUID of the user to update
   * @param quotaTier - New quota tier: 'Free' | 'Basic' | 'Pro' | 'Enterprise'
   * @returns Updated quota information
   *
   * @throws {Error} If user doesn't have superadmin role
   *
   * @example
   * const updatedQuota = await AdminSecurityService.updateUserQuota(
   *   'user-uuid-here',
   *   'Pro'
   * );
   */
  static async updateUserQuota(userId: string, quotaTier: string): Promise<UserQuotaInfo> {
    return apiClient.put<UserQuotaInfo>(
      `/api/admin/security/quotas/${userId}`,
      { quota_tier: quotaTier } as QuotaUpdateRequest
    );
  }

  // ==========================================================================
  // Encryption Key Rotation
  // ==========================================================================

  /**
   * GET /api/admin/security/encryption
   *
   * Fetches encryption key rotation status and history
   *
   * @returns Current active key, rotation status, all keys, and rotation history
   *
   * @example
   * const status = await AdminSecurityService.getEncryptionStatus();
   * if (status.rotation_status === 'OVERDUE') {
   *   console.warn('Encryption key rotation is overdue!');
   * }
   */
  static async getEncryptionStatus(): Promise<EncryptionStatus> {
    return apiClient.get<EncryptionStatus>('/api/admin/security/encryption');
  }

  /**
   * POST /api/admin/security/encryption/rotate
   *
   * Triggers manual encryption key rotation (Superadmin only)
   *
   * Creates a new encryption key and deprecates the current one.
   * This is a critical operation that should be performed with caution.
   *
   * @param reason - Optional reason for the rotation (for audit trail)
   * @returns Information about the newly created encryption key
   *
   * @throws {Error} If user doesn't have superadmin role
   *
   * @example
   * const newKey = await AdminSecurityService.rotateEncryptionKey(
   *   'Scheduled 90-day rotation'
   * );
   */
  static async rotateEncryptionKey(reason?: string): Promise<EncryptionKeyInfo> {
    return apiClient.post<EncryptionKeyInfo>(
      '/api/admin/security/encryption/rotate',
      { reason } as KeyRotationRequest
    );
  }

  // ==========================================================================
  // System Metrics
  // ==========================================================================

  /**
   * GET /api/admin/security/metrics
   *
   * Fetches Prometheus metrics summary for admin UI
   *
   * Note: Currently returns mock data. In production, this will be
   * populated from Prometheus/Grafana metrics scraping.
   *
   * @returns System metrics including request rates, latencies, pool stats
   *
   * @example
   * const metrics = await AdminSecurityService.getMetricsSummary();
   * console.log(`Active connections: ${metrics.active_connections}`);
   */
  static async getMetricsSummary(): Promise<MetricsSummary> {
    return apiClient.get<MetricsSummary>('/api/admin/security/metrics');
  }

  // ==========================================================================
  // Rate Limiting
  // ==========================================================================

  /**
   * GET /api/admin/security/rate-limits
   *
   * Fetches current rate limiting status
   *
   * Note: Rate limiting is in-memory, so this returns configuration
   * and any currently tracked limits.
   *
   * @returns Active rate limits, top limited IPs, and configuration
   *
   * @example
   * const rateLimits = await AdminSecurityService.getRateLimitStatus();
   * console.log(`Auth limit: ${rateLimits.configuration.auth_limit}`);
   */
  static async getRateLimitStatus(): Promise<RateLimitStatus> {
    return apiClient.get<RateLimitStatus>('/api/admin/security/rate-limits');
  }
}

// Export for convenience
export default AdminSecurityService;
