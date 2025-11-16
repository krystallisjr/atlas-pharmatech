// Alert & Notification Types

export interface AlertNotification {
  id: string;
  alert_type: string;
  severity: 'info' | 'warning' | 'critical';
  title: string;
  message: string;
  inventory_id?: string;
  metadata?: Record<string, any>;
  action_url?: string;
  is_read: boolean;
  created_at: string;
  time_ago: string;
}

export interface NotificationSummary {
  total_unread: number;
  total_notifications: number;
  notifications: AlertNotification[];
}

export interface UserAlertPreferences {
  user_id: string;
  expiry_alerts_enabled: boolean;
  expiry_alert_days: number;
  low_stock_alerts_enabled: boolean;
  low_stock_threshold: number;
  watchlist_alerts_enabled: boolean;
  email_notifications_enabled: boolean;
  in_app_notifications_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface UpdateAlertPreferencesRequest {
  expiry_alerts_enabled?: boolean;
  expiry_alert_days?: number;
  low_stock_alerts_enabled?: boolean;
  low_stock_threshold?: number;
  watchlist_alerts_enabled?: boolean;
  email_notifications_enabled?: boolean;
  in_app_notifications_enabled?: boolean;
}
