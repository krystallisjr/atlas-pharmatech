import { apiClient } from '../api-client';
import type { AlertNotification, NotificationSummary, UserAlertPreferences, UpdateAlertPreferencesRequest } from '@/types/alerts';

export class AlertService {
  static async getNotifications(params?: {
    limit?: number;
    offset?: number;
    unread_only?: boolean;
  }): Promise<NotificationSummary> {
    const queryParams = new URLSearchParams();
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());
    if (params?.unread_only) queryParams.append('unread_only', 'true');

    return await apiClient.get<NotificationSummary>(
      `/api/alerts/notifications?${queryParams.toString()}`
    );
  }

  static async getUnreadCount(): Promise<{ unread_count: number }> {
    return await apiClient.get('/api/alerts/notifications/unread-count');
  }

  static async markAsRead(notificationId: string, isRead: boolean = true): Promise<void> {
    await apiClient.put(`/api/alerts/notifications/${notificationId}/read`, { is_read: isRead });
  }

  static async markAllRead(): Promise<void> {
    await apiClient.post('/api/alerts/notifications/mark-all-read', {});
  }

  static async dismissNotification(notificationId: string): Promise<void> {
    await apiClient.delete(`/api/alerts/notifications/${notificationId}`);
  }

  static async getPreferences(): Promise<UserAlertPreferences> {
    return await apiClient.get('/api/alerts/preferences');
  }

  static async updatePreferences(data: UpdateAlertPreferencesRequest): Promise<UserAlertPreferences> {
    return await apiClient.put('/api/alerts/preferences', data);
  }

  // Watchlist methods
  static async getWatchlists(): Promise<any[]> {
    return await apiClient.get('/api/alerts/watchlist');
  }

  static async createWatchlist(data: any): Promise<any> {
    return await apiClient.post('/api/alerts/watchlist', data);
  }

  static async getWatchlist(id: string): Promise<any> {
    return await apiClient.get(`/api/alerts/watchlist/${id}`);
  }

  static async updateWatchlist(id: string, data: any): Promise<any> {
    return await apiClient.put(`/api/alerts/watchlist/${id}`, data);
  }

  static async deleteWatchlist(id: string): Promise<void> {
    await apiClient.delete(`/api/alerts/watchlist/${id}`);
  }
}
