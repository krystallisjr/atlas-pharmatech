'use client';

import { useEffect, useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { AdminSecurityService } from '@/lib/services/admin-security-service';
import { toast } from 'react-toastify';
import { format, subDays } from 'date-fns';
import {
  Area, AreaChart, Bar, BarChart, CartesianGrid, Cell, Legend, Pie, PieChart,
  ResponsiveContainer, Tooltip, XAxis, YAxis
} from 'recharts';
import {
  BarChart3,
  DollarSign,
  TrendingUp,
  Clock,
  AlertCircle,
  Filter,
  Search,
  ChevronLeft,
  ChevronRight,
  Download
} from 'lucide-react';
import type { ApiUsageAnalytics, ApiUsageFilters } from '@/types/admin-security';

const COLORS = ['#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#14B8A6'];

export default function ApiUsagePage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<ApiUsageAnalytics | null>(null);

  // Filters
  const [dateRange, setDateRange] = useState('30');  // last 30 days
  const [endpointFilter, setEndpointFilter] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 20;

  useEffect(() => {
    loadData();
  }, [dateRange, endpointFilter]);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);

      const endDate = new Date();
      const startDate = subDays(endDate, parseInt(dateRange));

      const filters: ApiUsageFilters = {
        start_date: startDate.toISOString(),
        end_date: endDate.toISOString(),
        limit: 100,
        offset: 0,
      };

      if (endpointFilter !== 'all') {
        filters.endpoint = endpointFilter;
      }

      const result = await AdminSecurityService.getApiUsageAnalytics(filters);
      setData(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load API usage data';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const clearFilters = () => {
    setDateRange('30');
    setEndpointFilter('all');
    setSearchQuery('');
    setCurrentPage(1);
  };

  const hasFilters = dateRange !== '30' || endpointFilter !== 'all' || searchQuery !== '';

  // Calculate pagination
  const filteredRequests = data?.recent_requests.filter(req =>
    req.user_email?.toLowerCase().includes(searchQuery.toLowerCase()) ||
    req.endpoint.toLowerCase().includes(searchQuery.toLowerCase())
  ) || [];

  const totalPages = Math.ceil(filteredRequests.length / itemsPerPage);
  const paginatedRequests = filteredRequests.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  );

  // Get unique endpoints for filter
  const uniqueEndpoints = Array.from(new Set(data?.usage_by_endpoint.map(e => e.endpoint) || []));

  // Loading state
  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading API usage data...</span>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  // Error state
  if (error || !data) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error || 'Failed to load data'}</p>
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
              <BarChart3 className="h-8 w-8 text-purple-600" />
              API Usage Analytics
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-2">
              Track Anthropic API costs, token usage, and performance metrics
            </p>
          </div>
          <Button variant="outline" onClick={() => toast.info('Export feature coming soon')}>
            <Download className="h-4 w-4 mr-2" />
            Export Report
          </Button>
        </div>

        {/* Summary Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Cost
              </CardTitle>
              <DollarSign className="h-5 w-5 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                ${(data.total_cost_cents / 100).toFixed(2)}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Last {dateRange} days
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Requests
              </CardTitle>
              <TrendingUp className="h-5 w-5 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {data.total_requests.toLocaleString()}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                API calls made
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Tokens
              </CardTitle>
              <BarChart3 className="h-5 w-5 text-purple-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {(data.total_tokens / 1000).toFixed(1)}K
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Input + Output tokens
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Avg Latency
              </CardTitle>
              <Clock className="h-5 w-5 text-orange-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {data.avg_latency_ms.toFixed(0)}ms
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Response time
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Filters */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Filter className="h-5 w-5" />
              Filters
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Date Range */}
              <div>
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">
                  Date Range
                </label>
                <Select value={dateRange} onValueChange={setDateRange}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="7">Last 7 days</SelectItem>
                    <SelectItem value="30">Last 30 days</SelectItem>
                    <SelectItem value="90">Last 90 days</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Endpoint Filter */}
              <div>
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">
                  Endpoint
                </label>
                <Select value={endpointFilter} onValueChange={setEndpointFilter}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Endpoints</SelectItem>
                    {uniqueEndpoints.map((endpoint) => (
                      <SelectItem key={endpoint} value={endpoint}>
                        {endpoint}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Search */}
              <div>
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">
                  Search
                </label>
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                  <Input
                    type="text"
                    placeholder="Search user or endpoint..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="pl-10"
                  />
                </div>
              </div>
            </div>

            <div className="mt-4 flex items-center justify-between">
              <div className="text-sm text-gray-600 dark:text-gray-400">
                Showing {paginatedRequests.length} of {filteredRequests.length} requests
              </div>
              {hasFilters && (
                <Button variant="link" size="sm" onClick={clearFilters}>
                  Clear filters
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Charts Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Usage Over Time */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Usage Over Time</CardTitle>
              <CardDescription>Daily API requests and costs</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <AreaChart data={data.usage_over_time}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis tick={{ fontSize: 12 }} />
                  <Tooltip />
                  <Legend />
                  <Area
                    type="monotone"
                    dataKey="requests"
                    stroke="#3B82F6"
                    fill="#3B82F6"
                    fillOpacity={0.6}
                    name="Requests"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Cost by Endpoint */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Cost by Endpoint</CardTitle>
              <CardDescription>Top endpoints by total cost</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={data.usage_by_endpoint.slice(0, 7)}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={({ endpoint, total_cost_cents }) =>
                      `${endpoint.split('/').pop()}: $${(total_cost_cents / 100).toFixed(2)}`
                    }
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="total_cost_cents"
                  >
                    {data.usage_by_endpoint.slice(0, 7).map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value: number) => `$${(value / 100).toFixed(2)}`}
                  />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Top Users by Usage */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Top Users</CardTitle>
              <CardDescription>Highest API consumers</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={data.usage_by_user.slice(0, 10)}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="user_email" tick={{ fontSize: 10 }} angle={-45} textAnchor="end" height={100} />
                  <YAxis tick={{ fontSize: 12 }} />
                  <Tooltip />
                  <Legend />
                  <Bar dataKey="request_count" fill="#10B981" name="Requests" />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Endpoint Performance */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Endpoint Performance</CardTitle>
              <CardDescription>Average latency by endpoint</CardDescription>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={data.usage_by_endpoint.slice(0, 10)}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="endpoint" tick={{ fontSize: 10 }} angle={-45} textAnchor="end" height={100} />
                  <YAxis tick={{ fontSize: 12 }} label={{ value: 'ms', angle: -90, position: 'insideLeft' }} />
                  <Tooltip />
                  <Legend />
                  <Bar dataKey="avg_latency_ms" fill="#F59E0B" name="Avg Latency (ms)" />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </div>

        {/* Recent Requests Table */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Recent API Requests</CardTitle>
            <CardDescription>Detailed log of recent API calls</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Timestamp</TableHead>
                    <TableHead>User</TableHead>
                    <TableHead>Endpoint</TableHead>
                    <TableHead className="text-right">Tokens</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-right">Latency</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginatedRequests.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center py-8 text-gray-500">
                        No requests found
                      </TableCell>
                    </TableRow>
                  ) : (
                    paginatedRequests.map((request) => (
                      <TableRow key={request.id} className="hover:bg-gray-50 dark:hover:bg-gray-800">
                        <TableCell className="text-sm text-gray-600 dark:text-gray-400">
                          {format(new Date(request.created_at), 'MMM dd, HH:mm:ss')}
                        </TableCell>
                        <TableCell className="text-sm font-medium">
                          {request.user_email || 'Unknown'}
                        </TableCell>
                        <TableCell className="text-sm">
                          <Badge variant="outline" className="font-mono text-xs">
                            {request.endpoint.split('/').pop()}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-right text-sm">
                          {request.total_tokens.toLocaleString()}
                        </TableCell>
                        <TableCell className="text-right text-sm font-medium">
                          ${(request.cost_cents / 100).toFixed(4)}
                        </TableCell>
                        <TableCell className="text-right text-sm">
                          <Badge
                            variant="secondary"
                            className={
                              request.latency_ms < 1000
                                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                : request.latency_ms < 3000
                                ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                            }
                          >
                            {request.latency_ms}ms
                          </Badge>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between p-4 border-t dark:border-gray-700">
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  Page {currentPage} of {totalPages}
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                    disabled={currentPage === 1}
                  >
                    <ChevronLeft className="h-4 w-4 mr-1" />
                    Previous
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                    disabled={currentPage === totalPages}
                  >
                    Next
                    <ChevronRight className="h-4 w-4 ml-1" />
                  </Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </DashboardLayout>
  );
}
