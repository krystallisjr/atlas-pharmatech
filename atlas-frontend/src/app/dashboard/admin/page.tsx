'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Users,
  UserCheck,
  UserCog,
  ShieldAlert,
  Package,
  Activity,
  Clock,
  CheckCircle2,
  AlertCircle,
  TrendingUp,
  Database,
} from 'lucide-react';
import { AdminService, AdminStats } from '@/lib/services/admin-service';
import { toast } from 'react-toastify';
import { formatDistanceToNow } from 'date-fns';

export default function AdminDashboardPage() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadStats();
  }, []);

  const loadStats = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await AdminService.getAdminStats();
      setStats(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load statistics';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div className="flex items-center justify-between">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              Admin Dashboard
            </h1>
          </div>
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

  if (error || !stats) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error || 'Failed to load dashboard'}</p>
              </div>
              <Button onClick={loadStats} className="mt-4" variant="outline">
                Retry
              </Button>
            </CardContent>
          </Card>
        </div>
      </DashboardLayout>
    );
  }

  const pendingPercentage = stats.total_users > 0
    ? (stats.pending_verifications / stats.total_users) * 100
    : 0;

  const verifiedPercentage = stats.total_users > 0
    ? (stats.verified_users / stats.total_users) * 100
    : 0;

  return (
    <DashboardLayout>
      <div className="p-8 space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              Admin Dashboard
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              System overview and statistics
            </p>
          </div>
          <Button onClick={loadStats} variant="outline">
            <Activity className="h-4 w-4 mr-2" />
            Refresh
          </Button>
        </div>

        {/* Quick Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {/* Total Users */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Users
              </CardTitle>
              <Users className="h-5 w-5 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {stats.total_users}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Registered accounts
              </p>
            </CardContent>
          </Card>

          {/* Verified Users */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Verified Users
              </CardTitle>
              <UserCheck className="h-5 w-5 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {stats.verified_users}
              </div>
              <div className="flex items-center gap-2 mt-1">
                <div className="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div
                    className="bg-green-600 h-2 rounded-full transition-all"
                    style={{ width: `${verifiedPercentage}%` }}
                  />
                </div>
                <span className="text-xs text-gray-500 dark:text-gray-500">
                  {verifiedPercentage.toFixed(0)}%
                </span>
              </div>
            </CardContent>
          </Card>

          {/* Pending Verifications */}
          <Link href="/dashboard/admin/verification">
            <Card className="hover:shadow-lg transition-shadow cursor-pointer hover:border-orange-400">
              <CardHeader className="pb-3 flex flex-row items-center justify-between">
                <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                  Pending Verifications
                </CardTitle>
                <ShieldAlert className="h-5 w-5 text-orange-600" />
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold text-orange-600">
                  {stats.pending_verifications}
                </div>
                <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                  Click to review →
                </p>
              </CardContent>
            </Card>
          </Link>

          {/* Admin Users */}
          <Card className="hover:shadow-lg transition-shadow">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Admin Users
              </CardTitle>
              <UserCog className="h-5 w-5 text-purple-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {stats.total_admins}
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-500 mt-1">
                Admin & Superadmin
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Secondary Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* System Stats */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Package className="h-5 w-5" />
                System Statistics
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-gray-600 dark:text-gray-400">Inventory Items</span>
                <span className="text-2xl font-bold text-gray-900 dark:text-white">
                  {stats.total_inventory_items.toLocaleString()}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-600 dark:text-gray-400">Total Transactions</span>
                <span className="text-2xl font-bold text-gray-900 dark:text-white">
                  {stats.total_transactions.toLocaleString()}
                </span>
              </div>
              <div className="pt-4 border-t dark:border-gray-700">
                <div className="flex items-center gap-2 text-sm">
                  <Database className={`h-4 w-4 ${stats.system_health.database_connected ? 'text-green-600' : 'text-red-600'}`} />
                  <span className="text-gray-600 dark:text-gray-400">Database Status:</span>
                  <Badge variant={stats.system_health.database_connected ? 'default' : 'destructive'} className={stats.system_health.database_connected ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : ''}>
                    {stats.system_health.database_connected ? 'Connected' : 'Disconnected'}
                  </Badge>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Recent Signups */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <TrendingUp className="h-5 w-5" />
                Recent Signups
              </CardTitle>
            </CardHeader>
            <CardContent>
              {stats.recent_signups.length === 0 ? (
                <p className="text-gray-500 dark:text-gray-500 text-center py-4">
                  No recent signups
                </p>
              ) : (
                <div className="space-y-3">
                  {stats.recent_signups.map((signup) => (
                    <Link
                      key={signup.id}
                      href={`/dashboard/admin/users/${signup.id}`}
                      className="block"
                    >
                      <div className="flex items-center justify-between p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors border dark:border-gray-700">
                        <div className="flex-1 min-w-0">
                          <p className="font-medium text-gray-900 dark:text-white truncate">
                            {signup.company_name}
                          </p>
                          <p className="text-sm text-gray-500 dark:text-gray-500 truncate">
                            {signup.email}
                          </p>
                        </div>
                        <div className="flex items-center gap-2 ml-4">
                          {signup.is_verified ? (
                            <CheckCircle2 className="h-4 w-4 text-green-600" />
                          ) : (
                            <Clock className="h-4 w-4 text-orange-600" />
                          )}
                          <span className="text-xs text-gray-500 dark:text-gray-500 whitespace-nowrap">
                            {formatDistanceToNow(new Date(signup.created_at), { addSuffix: true })}
                          </span>
                        </div>
                      </div>
                    </Link>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Quick Actions */}
        <Card>
          <CardHeader>
            <CardTitle>Quick Actions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <Link href="/dashboard/admin/users">
                <Button className="w-full" variant="outline">
                  <Users className="h-4 w-4 mr-2" />
                  Manage Users
                </Button>
              </Link>
              <Link href="/dashboard/admin/verification">
                <Button className="w-full" variant="outline">
                  <ShieldAlert className="h-4 w-4 mr-2" />
                  Verification Queue
                  {stats.pending_verifications > 0 && (
                    <Badge className="ml-2" variant="destructive">
                      {stats.pending_verifications}
                    </Badge>
                  )}
                </Button>
              </Link>
              <Link href="/dashboard/admin/audit-logs">
                <Button className="w-full" variant="outline">
                  <Activity className="h-4 w-4 mr-2" />
                  Audit Logs
                </Button>
              </Link>
            </div>
          </CardContent>
        </Card>
      </div>
    </DashboardLayout>
  );
}
