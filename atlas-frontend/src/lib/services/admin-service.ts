import { apiClient } from '../api-client';
import { User, UserRole } from '@/types/auth';

// ============================================================================
// Request Types
// ============================================================================

export interface ListUsersParams {
  limit?: number;
  offset?: number;
  role?: UserRole;
  verified?: boolean;
  search?: string;
}

export interface VerifyUserRequest {
  verified: boolean;
  notes?: string;
}

export interface ChangeUserRoleRequest {
  role: UserRole;
  notes?: string;
}

export interface AuditLogFilters {
  user_id?: string;
  event_category?: string;
  event_type?: string;
  severity?: 'info' | 'warning' | 'error' | 'critical';
  start_date?: string; // ISO 8601 date
  end_date?: string; // ISO 8601 date
  limit?: number;
  offset?: number;
}

// ============================================================================
// Response Types
// ============================================================================

export interface ListUsersResponse {
  users: User[];
  total: number;
  limit: number;
  offset: number;
}

export interface VerificationQueueItem {
  user: User;
  inventory_count: number;
  transaction_count: number;
  days_waiting: number;
}

export interface AdminStats {
  total_users: number;
  verified_users: number;
  pending_verifications: number;
  total_admins: number;
  total_inventory_items: number;
  total_transactions: number;
  recent_signups: RecentSignup[];
  system_health: SystemHealth;
}

export interface RecentSignup {
  id: string;
  email: string;
  company_name: string;
  created_at: string;
  is_verified: boolean;
}

export interface SystemHealth {
  database_connected: boolean;
  uptime_seconds: number;
  total_api_calls_today: number;
}

export interface AuditLog {
  id: string;
  event_type: string;
  event_category: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
  actor_user_id: string | null;
  action: string;
  action_result: string;
  event_data: Record<string, any>;
  ip_address: string | null;
  created_at: string;
}

// ============================================================================
// Admin Service Class
// ============================================================================

export class AdminService {
  /**
   * List all users with optional filtering and pagination
   * Requires: Admin or Superadmin role
   */
  static async listUsers(params?: ListUsersParams): Promise<ListUsersResponse> {
    const queryParams = new URLSearchParams();

    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());
    if (params?.role) queryParams.append('role', params.role);
    if (params?.verified !== undefined) queryParams.append('verified', params.verified.toString());
    if (params?.search) queryParams.append('search', params.search);

    const url = `/api/admin/users${queryParams.toString() ? `?${queryParams.toString()}` : ''}`;
    return apiClient.get<ListUsersResponse>(url);
  }

  /**
   * Get a specific user by ID
   * Requires: Admin or Superadmin role
   */
  static async getUser(userId: string): Promise<User> {
    return apiClient.get<User>(`/api/admin/users/${userId}`);
  }

  /**
   * Verify or unverify a user
   * Requires: Admin or Superadmin role
   */
  static async verifyUser(
    userId: string,
    verified: boolean,
    notes?: string
  ): Promise<User> {
    return apiClient.post<User>(
      `/api/admin/users/${userId}/verify`,
      { verified, notes }
    );
  }

  /**
   * Change user role
   * Requires: Superadmin role ONLY
   */
  static async changeUserRole(
    userId: string,
    role: UserRole,
    notes?: string
  ): Promise<User> {
    return apiClient.put<User>(
      `/api/admin/users/${userId}/role`,
      { role, notes }
    );
  }

  /**
   * Delete user (irreversible)
   * Requires: Superadmin role ONLY
   */
  static async deleteUser(userId: string): Promise<void> {
    return apiClient.delete<void>(`/api/admin/users/${userId}`);
  }

  /**
   * Get verification queue (unverified users)
   * Requires: Admin or Superadmin role
   */
  static async getVerificationQueue(): Promise<VerificationQueueItem[]> {
    return apiClient.get<VerificationQueueItem[]>('/api/admin/verification-queue');
  }

  /**
   * Get admin statistics dashboard data
   * Requires: Admin or Superadmin role
   */
  static async getAdminStats(): Promise<AdminStats> {
    return apiClient.get<AdminStats>('/api/admin/stats');
  }

  /**
   * Get audit logs with optional filters
   * Requires: Admin or Superadmin role
   */
  static async getAuditLogs(filters?: AuditLogFilters): Promise<AuditLog[]> {
    const queryParams = new URLSearchParams();

    if (filters?.user_id) queryParams.append('user_id', filters.user_id);
    if (filters?.event_category) queryParams.append('event_category', filters.event_category);
    if (filters?.event_type) queryParams.append('event_type', filters.event_type);
    if (filters?.severity) queryParams.append('severity', filters.severity);
    if (filters?.start_date) queryParams.append('start_date', filters.start_date);
    if (filters?.end_date) queryParams.append('end_date', filters.end_date);
    if (filters?.limit) queryParams.append('limit', filters.limit.toString());
    if (filters?.offset) queryParams.append('offset', filters.offset.toString());

    const url = `/api/admin/audit-logs${queryParams.toString() ? `?${queryParams.toString()}` : ''}`;
    return apiClient.get<AuditLog[]>(url);
  }

  /**
   * Check admin API health
   * Public endpoint - no auth required
   */
  static async checkHealth(): Promise<{ status: string; message: string }> {
    return apiClient.get<{ status: string; message: string }>('/api/admin/health');
  }
}
