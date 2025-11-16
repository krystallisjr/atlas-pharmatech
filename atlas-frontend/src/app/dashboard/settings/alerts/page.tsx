'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { AlertService } from '@/lib/services/alert-service';
import type { UserAlertPreferences, UpdateAlertPreferencesRequest } from '@/types/alerts';
import { toast } from 'react-toastify';
import { Bell, Package, Clock, Search, Mail, Smartphone, Loader2, ArrowLeft } from 'lucide-react';
import Link from 'next/link';

export default function AlertSettingsPage() {
  const [preferences, setPreferences] = useState<UserAlertPreferences | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    loadPreferences();
  }, []);

  const loadPreferences = async () => {
    try {
      setIsLoading(true);
      const data = await AlertService.getPreferences();
      setPreferences(data);
    } catch (error) {
      console.error('Failed to load preferences:', error);
      toast.error('Failed to load alert preferences');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    if (!preferences) return;

    try {
      setIsSaving(true);
      const request: UpdateAlertPreferencesRequest = {
        expiry_alerts_enabled: preferences.expiry_alerts_enabled,
        expiry_alert_days: preferences.expiry_alert_days,
        low_stock_alerts_enabled: preferences.low_stock_alerts_enabled,
        low_stock_threshold: preferences.low_stock_threshold,
        watchlist_alerts_enabled: preferences.watchlist_alerts_enabled,
        email_notifications_enabled: preferences.email_notifications_enabled,
        in_app_notifications_enabled: preferences.in_app_notifications_enabled,
      };

      const updated = await AlertService.updatePreferences(request);
      setPreferences(updated);
      toast.success('Alert preferences saved successfully');
    } catch (error) {
      console.error('Failed to save preferences:', error);
      toast.error('Failed to save preferences');
    } finally {
      setIsSaving(false);
    }
  };

  const updatePreference = <K extends keyof UserAlertPreferences>(
    key: K,
    value: UserAlertPreferences[K]
  ) => {
    if (!preferences) return;
    setPreferences({ ...preferences, [key]: value });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
      </div>
    );
  }

  if (!preferences) {
    return (
      <div className="text-center text-gray-500 py-12">
        Failed to load preferences
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <div>
        <Link href="/dashboard/settings">
          <Button variant="ghost" size="sm" className="mb-4">
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Settings
          </Button>
        </Link>
        <h1 className="text-3xl font-bold text-gray-900">Alert Preferences</h1>
        <p className="text-gray-600 mt-2">
          Configure how and when you receive notifications about your inventory
        </p>
      </div>

      {/* Alert Types */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Bell className="h-5 w-5" />
            Alert Types
          </CardTitle>
          <CardDescription>
            Choose which types of alerts you want to receive
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Expiry Alerts */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-yellow-600" />
                  <Label htmlFor="expiry-alerts" className="text-base font-medium">
                    Expiry Alerts
                  </Label>
                </div>
                <p className="text-sm text-gray-600">
                  Get notified when products are approaching their expiration date
                </p>
              </div>
              <Switch
                id="expiry-alerts"
                checked={preferences.expiry_alerts_enabled}
                onCheckedChange={(checked) => updatePreference('expiry_alerts_enabled', checked)}
              />
            </div>

            {preferences.expiry_alerts_enabled && (
              <div className="ml-6 space-y-2">
                <Label htmlFor="expiry-days" className="text-sm">
                  Alert me when products expire within
                </Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="expiry-days"
                    type="number"
                    min="1"
                    max="365"
                    value={preferences.expiry_alert_days}
                    onChange={(e) => updatePreference('expiry_alert_days', parseInt(e.target.value))}
                    className="w-24"
                  />
                  <span className="text-sm text-gray-600">days</span>
                </div>
                <p className="text-xs text-gray-500">
                  Critical alerts (7 days or less) will always be shown
                </p>
              </div>
            )}
          </div>

          <div className="border-t pt-6" />

          {/* Low Stock Alerts */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Package className="h-4 w-4 text-orange-600" />
                  <Label htmlFor="low-stock-alerts" className="text-base font-medium">
                    Low Stock Alerts
                  </Label>
                </div>
                <p className="text-sm text-gray-600">
                  Get notified when inventory quantities fall below your threshold
                </p>
              </div>
              <Switch
                id="low-stock-alerts"
                checked={preferences.low_stock_alerts_enabled}
                onCheckedChange={(checked) => updatePreference('low_stock_alerts_enabled', checked)}
              />
            </div>

            {preferences.low_stock_alerts_enabled && (
              <div className="ml-6 space-y-2">
                <Label htmlFor="stock-threshold" className="text-sm">
                  Alert me when stock falls below
                </Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="stock-threshold"
                    type="number"
                    min="0"
                    value={preferences.low_stock_threshold}
                    onChange={(e) => updatePreference('low_stock_threshold', parseInt(e.target.value))}
                    className="w-24"
                  />
                  <span className="text-sm text-gray-600">units</span>
                </div>
              </div>
            )}
          </div>

          <div className="border-t pt-6" />

          {/* Watchlist Alerts */}
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <Search className="h-4 w-4 text-blue-600" />
                <Label htmlFor="watchlist-alerts" className="text-base font-medium">
                  Watchlist Alerts
                </Label>
              </div>
              <p className="text-sm text-gray-600">
                Get notified when marketplace listings match your saved searches
              </p>
            </div>
            <Switch
              id="watchlist-alerts"
              checked={preferences.watchlist_alerts_enabled}
              onCheckedChange={(checked) => updatePreference('watchlist_alerts_enabled', checked)}
            />
          </div>
        </CardContent>
      </Card>

      {/* Notification Channels */}
      <Card>
        <CardHeader>
          <CardTitle>Notification Channels</CardTitle>
          <CardDescription>
            Choose how you want to receive notifications
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <Smartphone className="h-4 w-4 text-blue-600" />
                <Label htmlFor="in-app" className="text-base font-medium">
                  In-App Notifications
                </Label>
              </div>
              <p className="text-sm text-gray-600">
                Show notifications in the bell icon menu
              </p>
            </div>
            <Switch
              id="in-app"
              checked={preferences.in_app_notifications_enabled}
              onCheckedChange={(checked) => updatePreference('in_app_notifications_enabled', checked)}
            />
          </div>

          <div className="border-t pt-6" />

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <Mail className="h-4 w-4 text-green-600" />
                <Label htmlFor="email" className="text-base font-medium">
                  Email Notifications
                </Label>
              </div>
              <p className="text-sm text-gray-600">
                Receive alerts via email (coming soon)
              </p>
            </div>
            <Switch
              id="email"
              checked={preferences.email_notifications_enabled}
              onCheckedChange={(checked) => updatePreference('email_notifications_enabled', checked)}
              disabled
            />
          </div>
        </CardContent>
      </Card>

      {/* Save Button */}
      <div className="flex justify-end">
        <Button onClick={handleSave} disabled={isSaving} size="lg">
          {isSaving ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Saving...
            </>
          ) : (
            'Save Preferences'
          )}
        </Button>
      </div>
    </div>
  );
}
