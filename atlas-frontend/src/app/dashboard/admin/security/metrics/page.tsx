'use client';

import { useEffect, useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { AdminSecurityService } from '@/lib/services/admin-security-service';
import { toast } from 'react-toastify';
import {
  Bar, BarChart, CartesianGrid, Legend, Line, LineChart,
  ResponsiveContainer, Tooltip, XAxis, YAxis, Cell, PieChart, Pie
} from 'recharts';
import {
  Activity,
  AlertCircle,
  Clock,
  TrendingUp,
  Database,
  Zap,
  RefreshCw,
  AlertTriangle
} from 'lucide-react';
import type { MetricsSummary } from '@/types/admin-security';

const COLORS = ['#10B981', '#3B82F6', '#F59E0B', '#EF4444', '#8B5CF6'];

export default function MetricsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<MetricsSummary | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  useEffect(() => {
    loadData();

    if (autoRefresh) {
      const interval = setInterval(loadData, 30000); // Refresh every 30 seconds
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await AdminSecurityService.getMetricsSummary();
      setData(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load metrics';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  // Prepare chart data
  const latencyData = data ? [
    { name: 'P50', value: data.request_duration_p50 },
    { name: 'P95', value: data.request_duration_p95 },
    { name: 'P99', value: data.request_duration_p99 },
  ] : [];

  const poolData = data ? [
    { name: 'Active', value: data.db_pool_active, color: '#10B981' },
    { name: 'Idle', value: data.db_pool_idle, color: '#3B82F6' },
  ] : [];

  const statusCodeData = data
    ? Object.entries(data.status_code_breakdown).map(([code, count]) => ({
        code,
        count,
      }))
    : [];

  // Loading state
  if (loading && !data) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading system metrics...</span>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  // Error state
  if (error && !data) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error}</p>
              </div>
              <Button onClick={loadData} className="mt-4" variant="outline">
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
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-3">
              <Activity className="h-8 w-8 text-purple-600" />
              System Metrics
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-2">
              Real-time Prometheus metrics and system health monitoring
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAutoRefresh(!autoRefresh)}
            >
              <RefreshCw className={`h-4 w-4 mr-2 ${autoRefresh ? 'animate-spin' : ''}`} />
              {autoRefresh ? 'Auto-refresh ON' : 'Auto-refresh OFF'}
            </Button>
            <Button onClick={loadData} disabled={loading} size="sm">
              <RefreshCw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
              Refresh Now
            </Button>
          </div>
        </div>

        {/* Info Banner - Mock Data Warning */}
        {data && data.http_requests_total === 0 && (
          <Card className="border-blue-200 dark:border-blue-900 bg-blue-50 dark:bg-blue-900/20">
            <CardContent className="pt-6">
              <div className="flex items-start gap-3">
                <AlertTriangle className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                <div>
                  <h4 className="font-semibold text-blue-900 dark:text-blue-100">
                    Metrics Integration Pending
                  </h4>
                  <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                    The metrics endpoint is currently returning mock data. To enable real-time metrics,
                    integrate with Prometheus/Grafana by parsing the <code className="bg-blue-100 dark:bg-blue-800 px-1 rounded">/metrics</code> endpoint
                    or connecting to your Prometheus API.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Summary Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Requests
              </CardTitle>
              <TrendingUp className="h-5 w-5 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {data?.http_requests_total.toLocaleString() || 0}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {data?.http_requests_per_minute.toFixed(1) || 0} req/min
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Avg Response Time
              </CardTitle>
              <Clock className="h-5 w-5 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {data?.avg_request_duration_ms.toFixed(0) || 0}ms
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Average latency
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Active Connections
              </CardTitle>
              <Activity className="h-5 w-5 text-purple-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {data?.active_connections || 0}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Current connections
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Auth Failures
              </CardTitle>
              <AlertCircle className="h-5 w-5 text-red-600" />
            </CardHeader>
            <CardContent>
              <div className={`text-3xl font-bold ${(data?.auth_failures_total || 0) > 0 ? 'text-red-600' : 'text-gray-900 dark:text-white'}`}>
                {data?.auth_failures_total || 0}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                {data?.auth_failures_last_hour || 0} last hour
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Charts Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Request Latency Percentiles */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Request Latency Percentiles</CardTitle>
              <CardDescription>P50, P95, and P99 response times</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={latencyData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="name" />
                  <YAxis label={{ value: 'ms', angle: -90, position: 'insideLeft' }} />
                  <Tooltip />
                  <Legend />
                  <Bar dataKey="value" fill="#3B82F6" name="Latency (ms)" />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Database Pool Status */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Database className="h-5 w-5" />
                Database Connection Pool
              </CardTitle>
              <CardDescription>Active vs Idle connections</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={poolData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={({ name, value }) => `${name}: ${value}`}
                    outerRadius={100}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {poolData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Status Code Distribution */}
          {statusCodeData.length > 0 && (
            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle className="text-lg">HTTP Status Code Distribution</CardTitle>
                <CardDescription>Breakdown of response status codes</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <BarChart data={statusCodeData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="code" />
                    <YAxis />
                    <Tooltip />
                    <Legend />
                    <Bar dataKey="count" fill="#10B981" name="Count">
                      {statusCodeData.map((entry, index) => (
                        <Cell
                          key={`cell-${index}`}
                          fill={
                            entry.code.startsWith('2') ? '#10B981' :
                            entry.code.startsWith('3') ? '#3B82F6' :
                            entry.code.startsWith('4') ? '#F59E0B' :
                            '#EF4444'
                          }
                        />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>
          )}
        </div>

        {/* Detailed Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* HTTP Metrics */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Zap className="h-5 w-5" />
                HTTP Metrics
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Total Requests</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {data?.http_requests_total.toLocaleString() || 0}
                  </span>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Requests per Minute</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {data?.http_requests_per_minute.toFixed(2) || 0}
                  </span>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Avg Duration</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {data?.avg_request_duration_ms.toFixed(2) || 0}ms
                  </span>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Active Connections</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {data?.active_connections || 0}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Database & Auth Metrics */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Database className="h-5 w-5" />
                Database & Auth Metrics
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">DB Pool Active</span>
                  <Badge variant="default" className="bg-green-600">
                    {data?.db_pool_active || 0}
                  </Badge>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">DB Pool Idle</span>
                  <Badge variant="secondary">
                    {data?.db_pool_idle || 0}
                  </Badge>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Total Auth Failures</span>
                  <Badge variant={data && data.auth_failures_total > 0 ? 'destructive' : 'secondary'}>
                    {data?.auth_failures_total || 0}
                  </Badge>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Auth Failures (1h)</span>
                  <Badge variant={data && data.auth_failures_last_hour > 0 ? 'destructive' : 'secondary'}>
                    {data?.auth_failures_last_hour || 0}
                  </Badge>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Latency Percentiles */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                <Clock className="h-5 w-5" />
                Latency Percentiles
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">P50 (Median)</span>
                  <Badge
                    variant="secondary"
                    className={
                      (data?.request_duration_p50 || 0) < 100
                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                        : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                    }
                  >
                    {data?.request_duration_p50.toFixed(2) || 0}ms
                  </Badge>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">P95</span>
                  <Badge
                    variant="secondary"
                    className={
                      (data?.request_duration_p95 || 0) < 500
                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                        : 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                    }
                  >
                    {data?.request_duration_p95.toFixed(2) || 0}ms
                  </Badge>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">P99</span>
                  <Badge
                    variant="secondary"
                    className={
                      (data?.request_duration_p99 || 0) < 1000
                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                        : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                    }
                  >
                    {data?.request_duration_p99.toFixed(2) || 0}ms
                  </Badge>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </DashboardLayout>
  );
}
