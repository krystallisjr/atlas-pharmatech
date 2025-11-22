'use client';

import { useEffect, useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Activity,
  Filter,
  Search,
  AlertCircle,
  AlertTriangle,
  Info,
  XCircle,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { AdminService, AuditLog, AuditLogFilters } from '@/lib/services/admin-service';
import { toast } from 'react-toastify';
import { format } from 'date-fns';

const LIMIT = 100;

export default function AuditLogsPage() {
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [eventCategory, setEventCategory] = useState<string>('all');
  const [severity, setSeverity] = useState<string>('all');
  const [userIdFilter, setUserIdFilter] = useState('');
  const [expandedLogs, setExpandedLogs] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadLogs();
  }, [eventCategory, severity, userIdFilter]);

  const loadLogs = async () => {
    try {
      setLoading(true);
      setError(null);

      const filters: AuditLogFilters = {
        limit: LIMIT,
        event_category: eventCategory !== 'all' ? eventCategory : undefined,
        severity: severity !== 'all' ? (severity as any) : undefined,
        user_id: userIdFilter || undefined,
      };

      const data = await AdminService.getAuditLogs(filters);
      setLogs(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load audit logs';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const toggleExpanded = (logId: string) => {
    setExpandedLogs((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(logId)) {
        newSet.delete(logId);
      } else {
        newSet.add(logId);
      }
      return newSet;
    });
  };

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'critical':
        return <XCircle className="h-4 w-4" />;
      case 'error':
        return <XCircle className="h-4 w-4" />;
      case 'warning':
        return <AlertTriangle className="h-4 w-4" />;
      default:
        return <Info className="h-4 w-4" />;
    }
  };

  const getSeverityBadgeVariant = (severity: string): 'default' | 'secondary' | 'destructive' | 'outline' => {
    switch (severity) {
      case 'critical':
        return 'destructive';
      case 'error':
        return 'destructive';
      case 'warning':
        return 'outline';
      default:
        return 'secondary';
    }
  };

  const getCategoryColor = (category: string): string => {
    switch (category) {
      case 'admin':
        return 'text-purple-600 bg-purple-100 dark:bg-purple-900 dark:text-purple-300';
      case 'security':
        return 'text-red-600 bg-red-100 dark:bg-red-900 dark:text-red-300';
      case 'authentication':
        return 'text-blue-600 bg-blue-100 dark:bg-blue-900 dark:text-blue-300';
      default:
        return 'text-gray-600 bg-gray-100 dark:bg-gray-800 dark:text-gray-300';
    }
  };

  return (
    <DashboardLayout>
      <div className="p-8 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
              <Activity className="h-8 w-8" />
              Audit Logs
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              View and filter system audit trail
            </p>
          </div>
          <Button onClick={loadLogs} variant="outline">
            Refresh
          </Button>
        </div>

        {/* Filters Card */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Filter className="h-5 w-5" />
              Filters
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Category Filter */}
              <Select value={eventCategory} onValueChange={setEventCategory}>
                <SelectTrigger>
                  <SelectValue placeholder="Filter by category" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Categories</SelectItem>
                  <SelectItem value="admin">Admin Actions</SelectItem>
                  <SelectItem value="security">Security</SelectItem>
                  <SelectItem value="authentication">Authentication</SelectItem>
                  <SelectItem value="user">User Actions</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>

              {/* Severity Filter */}
              <Select value={severity} onValueChange={setSeverity}>
                <SelectTrigger>
                  <SelectValue placeholder="Filter by severity" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Severities</SelectItem>
                  <SelectItem value="info">Info</SelectItem>
                  <SelectItem value="warning">Warning</SelectItem>
                  <SelectItem value="error">Error</SelectItem>
                  <SelectItem value="critical">Critical</SelectItem>
                </SelectContent>
              </Select>

              {/* User ID Filter */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  type="text"
                  placeholder="Filter by User ID..."
                  value={userIdFilter}
                  onChange={(e) => setUserIdFilter(e.target.value)}
                  className="pl-10"
                />
              </div>
            </div>

            {/* Results count */}
            <div className="mt-4 text-sm text-gray-600 dark:text-gray-400">
              Showing {logs.length} log entries (limited to last {LIMIT})
              {(eventCategory !== 'all' || severity !== 'all' || userIdFilter) && (
                <Button
                  variant="link"
                  size="sm"
                  onClick={() => {
                    setEventCategory('all');
                    setSeverity('all');
                    setUserIdFilter('');
                  }}
                  className="ml-2"
                >
                  Clear filters
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Logs List */}
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading audit logs...</span>
          </div>
        ) : error ? (
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error}</p>
              </div>
              <Button onClick={loadLogs} className="mt-4" variant="outline">
                Retry
              </Button>
            </CardContent>
          </Card>
        ) : logs.length === 0 ? (
          <Card>
            <CardContent className="py-12">
              <div className="text-center">
                <Activity className="h-16 w-16 text-gray-400 mx-auto mb-4 opacity-20" />
                <p className="text-gray-600 dark:text-gray-400">No audit logs found</p>
                {(eventCategory !== 'all' || severity !== 'all' || userIdFilter) && (
                  <p className="text-sm text-gray-500 mt-1">Try adjusting your filters</p>
                )}
              </div>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-2">
            {logs.map((log) => {
              const isExpanded = expandedLogs.has(log.id);

              return (
                <Card
                  key={log.id}
                  className="hover:shadow-md transition-shadow cursor-pointer"
                  onClick={() => toggleExpanded(log.id)}
                >
                  <CardContent className="p-4">
                    <div className="flex items-start justify-between gap-4">
                      {/* Main Log Info */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-3 flex-wrap">
                          {/* Expand Icon */}
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4 text-gray-400 flex-shrink-0" />
                          ) : (
                            <ChevronRight className="h-4 w-4 text-gray-400 flex-shrink-0" />
                          )}

                          {/* Severity Badge */}
                          <Badge
                            variant={getSeverityBadgeVariant(log.severity)}
                            className="flex items-center gap-1"
                          >
                            {getSeverityIcon(log.severity)}
                            {log.severity}
                          </Badge>

                          {/* Category Badge */}
                          <Badge className={getCategoryColor(log.event_category)}>
                            {log.event_category}
                          </Badge>

                          {/* Event Type */}
                          <span className="font-medium text-gray-900 dark:text-white truncate">
                            {log.event_type}
                          </span>

                          {/* Action */}
                          <span className="text-sm text-gray-600 dark:text-gray-400 truncate">
                            {log.action}
                          </span>

                          {/* Result */}
                          <Badge
                            variant={log.action_result === 'success' ? 'default' : 'destructive'}
                            className="text-xs"
                          >
                            {log.action_result}
                          </Badge>
                        </div>

                        {/* Timestamp */}
                        <div className="mt-2 text-xs text-gray-500 dark:text-gray-500 ml-7">
                          {format(new Date(log.created_at), 'PPpp')}
                          {log.actor_user_id && (
                            <span className="ml-4">
                              User: <code className="text-xs bg-gray-100 dark:bg-gray-800 px-1 py-0.5 rounded">{log.actor_user_id}</code>
                            </span>
                          )}
                          {log.ip_address && (
                            <span className="ml-4">
                              IP: <code className="text-xs bg-gray-100 dark:bg-gray-800 px-1 py-0.5 rounded">{log.ip_address}</code>
                            </span>
                          )}
                        </div>

                        {/* Expanded Details */}
                        {isExpanded && (
                          <div className="mt-4 ml-7 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                            <h4 className="text-sm font-semibold text-gray-900 dark:text-white mb-2">
                              Event Data
                            </h4>
                            <pre className="text-xs text-gray-700 dark:text-gray-300 overflow-x-auto whitespace-pre-wrap break-words">
                              {JSON.stringify(log.event_data, null, 2)}
                            </pre>
                          </div>
                        )}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}

        {/* Info Banner */}
        {logs.length >= LIMIT && (
          <Card className="border-blue-200 dark:border-blue-900 bg-blue-50 dark:bg-blue-900/20">
            <CardContent className="py-4">
              <div className="flex items-center gap-3 text-blue-700 dark:text-blue-300">
                <Info className="h-5 w-5 flex-shrink-0" />
                <p className="text-sm">
                  Showing the most recent {LIMIT} log entries. Use filters to refine results or contact support for historical data.
                </p>
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </DashboardLayout>
  );
}
