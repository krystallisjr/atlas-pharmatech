'use client';

import { useEffect, useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { AdminSecurityService } from '@/lib/services/admin-security-service';
import { toast } from 'react-toastify';
import {
  Users,
  AlertCircle,
  Search,
  Filter,
  TrendingUp,
  ChevronLeft,
  ChevronRight,
  AlertTriangle,
  CheckCircle2
} from 'lucide-react';
import type { UserQuotaInfo, QuotaTier } from '@/types/admin-security';

export default function QuotaManagementPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [quotas, setQuotas] = useState<UserQuotaInfo[]>([]);
  const [filteredQuotas, setFilteredQuotas] = useState<UserQuotaInfo[]>([]);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [tierFilter, setTierFilter] = useState<string>('all');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 20;

  // Tier upgrade dialog
  const [upgradeDialog, setUpgradeDialog] = useState<{
    open: boolean;
    user: UserQuotaInfo | null;
    newTier: QuotaTier | null;
  }>({
    open: false,
    user: null,
    newTier: null,
  });
  const [upgrading, setUpgrading] = useState(false);

  useEffect(() => {
    loadQuotas();
  }, []);

  useEffect(() => {
    filterQuotas();
  }, [quotas, searchQuery, tierFilter, statusFilter]);

  const loadQuotas = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await AdminSecurityService.getUserQuotas();
      setQuotas(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load quota data';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const filterQuotas = () => {
    let filtered = [...quotas];

    // Search filter
    if (searchQuery) {
      filtered = filtered.filter(q =>
        q.user_email.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    // Tier filter
    if (tierFilter !== 'all') {
      filtered = filtered.filter(q => q.quota_tier === tierFilter);
    }

    // Status filter
    if (statusFilter === 'over_quota') {
      filtered = filtered.filter(q => q.is_over_quota);
    } else if (statusFilter === 'high_usage') {
      filtered = filtered.filter(q => q.usage_percent > 80 && !q.is_over_quota);
    } else if (statusFilter === 'normal') {
      filtered = filtered.filter(q => q.usage_percent <= 80);
    }

    setFilteredQuotas(filtered);
    setCurrentPage(1);
  };

  const clearFilters = () => {
    setSearchQuery('');
    setTierFilter('all');
    setStatusFilter('all');
    setCurrentPage(1);
  };

  const hasFilters = searchQuery !== '' || tierFilter !== 'all' || statusFilter !== 'all';

  const handleTierChange = (user: UserQuotaInfo, newTier: QuotaTier) => {
    if (newTier === user.quota_tier) return;

    setUpgradeDialog({
      open: true,
      user,
      newTier,
    });
  };

  const confirmTierUpgrade = async () => {
    if (!upgradeDialog.user || !upgradeDialog.newTier) return;

    try {
      setUpgrading(true);
      const updated = await AdminSecurityService.updateUserQuota(
        upgradeDialog.user.user_id,
        upgradeDialog.newTier
      );

      // Update local state
      setQuotas(prev => prev.map(q =>
        q.user_id === updated.user_id ? updated : q
      ));

      toast.success(`Updated ${upgradeDialog.user.user_email} to ${upgradeDialog.newTier} tier`);
      setUpgradeDialog({ open: false, user: null, newTier: null });
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update quota';
      toast.error(message);
    } finally {
      setUpgrading(false);
    }
  };

  // Calculate stats
  const stats = {
    total: quotas.length,
    overQuota: quotas.filter(q => q.is_over_quota).length,
    highUsage: quotas.filter(q => q.usage_percent > 80 && !q.is_over_quota).length,
    totalCost: quotas.reduce((sum, q) => sum + q.total_cost_cents, 0),
  };

  // Pagination
  const totalPages = Math.ceil(filteredQuotas.length / itemsPerPage);
  const paginatedQuotas = filteredQuotas.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  );

  // Get usage badge
  const getUsageBadge = (quota: UserQuotaInfo) => {
    if (quota.is_over_quota) {
      return (
        <Badge variant="destructive" className="gap-1">
          <AlertTriangle className="h-3 w-3" />
          Over Quota
        </Badge>
      );
    }

    if (quota.usage_percent > 80) {
      return (
        <Badge variant="outline" className="border-orange-300 text-orange-700 dark:border-orange-700 dark:text-orange-400 gap-1">
          <AlertCircle className="h-3 w-3" />
          High Usage
        </Badge>
      );
    }

    return (
      <Badge variant="secondary" className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 gap-1">
        <CheckCircle2 className="h-3 w-3" />
        Normal
      </Badge>
    );
  };

  // Get progress bar color
  const getProgressColor = (percent: number) => {
    if (percent >= 100) return 'bg-red-600';
    if (percent >= 80) return 'bg-orange-500';
    if (percent >= 50) return 'bg-yellow-500';
    return 'bg-green-600';
  };

  // Loading state
  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading quota data...</span>
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
              <Button onClick={loadQuotas} className="mt-4" variant="outline">
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
            <Users className="h-8 w-8 text-purple-600" />
            Quota Management
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-2">
            Manage user API quotas and monitor usage limits
          </p>
        </div>

        {/* Summary Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Users
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                {stats.total}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Over Quota
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className={`text-3xl font-bold ${stats.overQuota > 0 ? 'text-red-600' : 'text-gray-900 dark:text-white'}`}>
                {stats.overQuota}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                High Usage (&gt;80%)
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className={`text-3xl font-bold ${stats.highUsage > 0 ? 'text-orange-600' : 'text-gray-900 dark:text-white'}`}>
                {stats.highUsage}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-gray-600 dark:text-gray-400">
                Total Monthly Cost
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900 dark:text-white">
                ${(stats.totalCost / 100).toFixed(2)}
              </div>
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
              {/* Search */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  type="text"
                  placeholder="Search by email..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10"
                />
              </div>

              {/* Tier Filter */}
              <Select value={tierFilter} onValueChange={setTierFilter}>
                <SelectTrigger>
                  <SelectValue placeholder="All Tiers" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Tiers</SelectItem>
                  <SelectItem value="Free">Free</SelectItem>
                  <SelectItem value="Basic">Basic</SelectItem>
                  <SelectItem value="Pro">Pro</SelectItem>
                  <SelectItem value="Enterprise">Enterprise</SelectItem>
                </SelectContent>
              </Select>

              {/* Status Filter */}
              <Select value={statusFilter} onValueChange={setStatusFilter}>
                <SelectTrigger>
                  <SelectValue placeholder="All Status" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Status</SelectItem>
                  <SelectItem value="over_quota">Over Quota</SelectItem>
                  <SelectItem value="high_usage">High Usage (&gt;80%)</SelectItem>
                  <SelectItem value="normal">Normal</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="mt-4 flex items-center justify-between">
              <div className="text-sm text-gray-600 dark:text-gray-400">
                Showing {paginatedQuotas.length} of {filteredQuotas.length} users
              </div>
              {hasFilters && (
                <Button variant="link" size="sm" onClick={clearFilters}>
                  Clear filters
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Quota Table */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">User Quotas</CardTitle>
            <CardDescription>Manage and monitor API usage quotas for all users</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>User</TableHead>
                    <TableHead>Tier</TableHead>
                    <TableHead>Usage</TableHead>
                    <TableHead className="text-right">Used / Limit</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-center">Status</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginatedQuotas.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center py-8 text-gray-500">
                        No users found
                      </TableCell>
                    </TableRow>
                  ) : (
                    paginatedQuotas.map((quota) => (
                      <TableRow key={quota.user_id} className="hover:bg-gray-50 dark:hover:bg-gray-800">
                        <TableCell className="font-medium">
                          {quota.user_email}
                        </TableCell>

                        <TableCell>
                          <Select
                            value={quota.quota_tier}
                            onValueChange={(value) => handleTierChange(quota, value as QuotaTier)}
                          >
                            <SelectTrigger className="w-32">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="Free">Free</SelectItem>
                              <SelectItem value="Basic">Basic</SelectItem>
                              <SelectItem value="Pro">Pro</SelectItem>
                              <SelectItem value="Enterprise">Enterprise</SelectItem>
                            </SelectContent>
                          </Select>
                        </TableCell>

                        <TableCell>
                          <div className="space-y-1">
                            <div className="flex items-center justify-between text-xs text-gray-600 dark:text-gray-400">
                              <span>{quota.usage_percent.toFixed(1)}%</span>
                            </div>
                            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
                              <div
                                className={`h-full transition-all ${getProgressColor(quota.usage_percent)}`}
                                style={{ width: `${Math.min(quota.usage_percent, 100)}%` }}
                              />
                            </div>
                          </div>
                        </TableCell>

                        <TableCell className="text-right text-sm">
                          <div className="font-medium">
                            {quota.monthly_used.toLocaleString()}
                            {quota.monthly_limit !== null && (
                              <span className="text-gray-500 dark:text-gray-400">
                                {' / '}
                                {quota.monthly_limit.toLocaleString()}
                              </span>
                            )}
                            {quota.monthly_limit === null && (
                              <span className="text-gray-500 dark:text-gray-400"> / ∞</span>
                            )}
                          </div>
                          {quota.monthly_remaining !== null && (
                            <div className="text-xs text-gray-500 dark:text-gray-400">
                              {quota.monthly_remaining.toLocaleString()} remaining
                            </div>
                          )}
                        </TableCell>

                        <TableCell className="text-right text-sm font-medium">
                          ${(quota.total_cost_cents / 100).toFixed(2)}
                        </TableCell>

                        <TableCell className="text-center">
                          {getUsageBadge(quota)}
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

        {/* Tier Upgrade Confirmation Dialog */}
        <AlertDialog open={upgradeDialog.open} onOpenChange={(open) => !upgrading && setUpgradeDialog({ ...upgradeDialog, open })}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Update Quota Tier</AlertDialogTitle>
              <AlertDialogDescription>
                Are you sure you want to change <strong>{upgradeDialog.user?.user_email}</strong> from{' '}
                <strong>{upgradeDialog.user?.quota_tier}</strong> to{' '}
                <strong>{upgradeDialog.newTier}</strong> tier?
                <br /><br />
                This will update their monthly API request limit immediately.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={upgrading}>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={confirmTierUpgrade} disabled={upgrading}>
                {upgrading ? 'Updating...' : 'Confirm Update'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </DashboardLayout>
  );
}
