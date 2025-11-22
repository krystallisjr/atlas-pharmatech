'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { AdminSecurityService } from '@/lib/services/admin-security-service';
import { toast } from 'react-toastify';
import {
  Lock,
  DollarSign,
  Activity,
  Key,
  TrendingUp,
  AlertCircle,
  ChevronRight,
  BarChart3,
  Users,
  ShieldCheck,
  Clock
} from 'lucide-react';
import type {
  ApiUsageAnalytics,
  UserQuotaInfo,
  EncryptionStatus,
  MetricsSummary
} from '@/types/admin-security';

export default function SecurityOverviewPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Overview stats from all security features
  const [apiUsage, setApiUsage] = useState<ApiUsageAnalytics | null>(null);
  const [quotas, setQuotas] = useState<UserQuotaInfo[]>([]);
  const [encryptionStatus, setEncryptionStatus] = useState<EncryptionStatus | null>(null);
  const [metrics, setMetrics] = useState<MetricsSummary | null>(null);

  useEffect(() => {
    loadOverviewData();
  }, []);

  const loadOverviewData = async () => {
    try {
      setLoading(true);
      setError(null);

      // Load data from all security features in parallel
      const [usageData, quotaData, encData, metricsData] = await Promise.allSettled([
        AdminSecurityService.getApiUsageAnalytics({ limit: 10 }),
        AdminSecurityService.getUserQuotas(),
        AdminSecurityService.getEncryptionStatus(),
        AdminSecurityService.getMetricsSummary(),
      ]);

      if (usageData.status === 'fulfilled') {
        setApiUsage(usageData.value);
      }
      if (quotaData.status === 'fulfilled') {
        setQuotas(quotaData.value);
      }
      if (encData.status === 'fulfilled') {
        setEncryptionStatus(encData.value);
      }
      if (metricsData.status === 'fulfilled') {
        setMetrics(metricsData.value);
      }

    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load security overview';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  // Calculate derived stats
  const totalCostDollars = apiUsage ? (apiUsage.total_cost_cents / 100).toFixed(2) : '0.00';
  const overQuotaCount = quotas.filter(q => q.is_over_quota).length;
  const highUsageCount = quotas.filter(q => q.usage_percent > 80 && !q.is_over_quota).length;

  // Encryption status badge
  const getEncryptionStatusBadge = () => {
    if (!encryptionStatus) return null;

    const { rotation_status, days_until_rotation } = encryptionStatus;

    if (rotation_status === 'OVERDUE') {
      return (
        <Badge variant="destructive" className="ml-2">
          <AlertCircle className="h-3 w-3 mr-1" />
          OVERDUE
        </Badge>
      );
    }

    if (rotation_status === 'SOON') {
      return (
        <Badge variant="outline" className="border-orange-300 text-orange-700 dark:border-orange-700 dark:text-orange-400 ml-2">
          <Clock className="h-3 w-3 mr-1" />
          {days_until_rotation}d remaining
        </Badge>
      );
    }

    return (
      <Badge variant="secondary" className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 ml-2">
        <ShieldCheck className="h-3 w-3 mr-1" />
        OK
      </Badge>
    );
  };

  // Loading state
  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-3">
              <Lock className="h-8 w-8 text-purple-600" />
              Security Monitoring
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-2">
              Comprehensive security and usage monitoring dashboard
            </p>
          </div>

          {/* Loading Skeleton */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {[1, 2, 3, 4].map((i) => (
              <Card key={i} className="animate-pulse">
                <CardHeader className="pb-3">
                  <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                </CardHeader>
                <CardContent>
                  <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </DashboardLayout>
    );
  }

  // Error state
  if (error) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error}</p>
              </div>
              <Button onClick={loadOverviewData} className="mt-4" variant="outline">
                Retry
              </Button>
            </CardContent>
          </Card>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="p-8 space-y-8">
        {/* Header */}
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-3">
            <Lock className="h-8 w-8 text-purple-600" />
            Security Monitoring
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-2">
            Monitor API usage, quotas, encryption keys, and system metrics
          </p>
        </div>

        {/* Quick Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {/* Total API Cost */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total API Cost
              </CardTitle>
              <DollarSign className="h-5 w-5 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                ${totalCostDollars}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {apiUsage?.total_requests.toLocaleString() || 0} requests
              </p>
            </CardContent>
          </Card>

          {/* Users Over Quota */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Quota Alerts
              </CardTitle>
              <TrendingUp className="h-5 w-5 text-orange-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {overQuotaCount}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {highUsageCount} near limit (>80%)
              </p>
            </CardContent>
          </Card>

          {/* Encryption Key Status */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400 flex items-center">
                Encryption Key
                {getEncryptionStatusBadge()}
              </CardTitle>
              <Key className="h-5 w-5 text-purple-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                v{encryptionStatus?.active_key.key_version || '1'}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {encryptionStatus?.active_key.age_days || 0} days old
              </p>
            </CardContent>
          </Card>

          {/* System Health */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Active Connections
              </CardTitle>
              <Activity className="h-5 w-5 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {metrics?.active_connections || 0}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {metrics?.auth_failures_total || 0} auth failures
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Feature Cards - Links to detailed pages */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* API Usage Monitoring */}
          <Link href="/dashboard/admin/security/api-usage" className="block group">
            <Card className="hover:shadow-lg transition-all hover:border-purple-200 dark:hover:border-purple-800">
              <CardHeader>
                <CardTitle className="flex items-center justify-between text-lg">
                  <div className="flex items-center gap-2">
                    <BarChart3 className="h-5 w-5 text-purple-600" />
                    API Usage Analytics
                  </div>
                  <ChevronRight className="h-5 w-5 text-gray-400 group-hover:text-purple-600 transition-colors" />
                </CardTitle>
                <CardDescription>
                  Track API costs, token usage, and endpoint performance
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Total Requests</span>
                    <span className="font-semibold">{apiUsage?.total_requests.toLocaleString() || 0}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Total Tokens</span>
                    <span className="font-semibold">{apiUsage?.total_tokens.toLocaleString() || 0}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Avg Latency</span>
                    <span className="font-semibold">{apiUsage?.avg_latency_ms.toFixed(0) || 0}ms</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>

          {/* Quota Management */}
          <Link href="/dashboard/admin/security/quotas" className="block group">
            <Card className="hover:shadow-lg transition-all hover:border-purple-200 dark:hover:border-purple-800">
              <CardHeader>
                <CardTitle className="flex items-center justify-between text-lg">
                  <div className="flex items-center gap-2">
                    <Users className="h-5 w-5 text-blue-600" />
                    Quota Management
                  </div>
                  <ChevronRight className="h-5 w-5 text-gray-400 group-hover:text-purple-600 transition-colors" />
                </CardTitle>
                <CardDescription>
                  Manage user API quotas and monitor usage limits
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Total Users</span>
                    <span className="font-semibold">{quotas.length}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Over Quota</span>
                    <span className={`font-semibold ${overQuotaCount > 0 ? 'text-red-600' : ''}`}>
                      {overQuotaCount}
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">High Usage (>80%)</span>
                    <span className={`font-semibold ${highUsageCount > 0 ? 'text-orange-600' : ''}`}>
                      {highUsageCount}
                    </span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>

          {/* Encryption Key Rotation */}
          <Link href="/dashboard/admin/security/encryption" className="block group">
            <Card className="hover:shadow-lg transition-all hover:border-purple-200 dark:hover:border-purple-800">
              <CardHeader>
                <CardTitle className="flex items-center justify-between text-lg">
                  <div className="flex items-center gap-2">
                    <Key className="h-5 w-5 text-green-600" />
                    Encryption Key Rotation
                  </div>
                  <ChevronRight className="h-5 w-5 text-gray-400 group-hover:text-purple-600 transition-colors" />
                </CardTitle>
                <CardDescription>
                  Monitor key lifecycle and trigger manual rotations
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Current Version</span>
                    <span className="font-semibold">v{encryptionStatus?.active_key.key_version || 1}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Status</span>
                    {getEncryptionStatusBadge()}
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Days Until Rotation</span>
                    <span className="font-semibold">{encryptionStatus?.days_until_rotation || 0}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>

          {/* System Metrics */}
          <Link href="/dashboard/admin/security/metrics" className="block group">
            <Card className="hover:shadow-lg transition-all hover:border-purple-200 dark:hover:border-purple-800">
              <CardHeader>
                <CardTitle className="flex items-center justify-between text-lg">
                  <div className="flex items-center gap-2">
                    <Activity className="h-5 w-5 text-orange-600" />
                    System Metrics
                  </div>
                  <ChevronRight className="h-5 w-5 text-gray-400 group-hover:text-purple-600 transition-colors" />
                </CardTitle>
                <CardDescription>
                  Real-time system health and performance metrics
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Requests/Min</span>
                    <span className="font-semibold">{metrics?.http_requests_per_minute.toFixed(1) || 0}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Avg Response Time</span>
                    <span className="font-semibold">{metrics?.avg_request_duration_ms.toFixed(0) || 0}ms</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">DB Pool Active</span>
                    <span className="font-semibold">{metrics?.db_pool_active || 0}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>
        </div>
      </div>
    </DashboardLayout>
  );
}
