'use client';

import { use, useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, RefreshCw, Filter, Loader2, Calendar } from 'lucide-react';
import { ErpService } from '@/lib/services';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { ErpConnection, SyncLog } from '@/types/erp';
import { toast } from 'react-toastify';
import { SyncTriggerButton } from '@/components/erp/sync/SyncTriggerButton';
import { SyncHistoryTable } from '@/components/erp/sync/SyncHistoryTable';
import { AiErrorAnalysis } from '@/components/erp/sync/AiErrorAnalysis';

type StatusFilter = 'all' | 'completed' | 'failed' | 'partial' | 'in_progress' | 'pending';

export default function SyncLogsPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const router = useRouter();
  const [connection, setConnection] = useState<ErpConnection | null>(null);
  const [syncLogs, setSyncLogs] = useState<SyncLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);
  const [showAiAnalysis, setShowAiAnalysis] = useState(false);

  useEffect(() => {
    loadData();
  }, [id]);

  const loadData = async () => {
    try {
      setLoading(true);
      const [connData, logsData] = await Promise.all([
        ErpService.getConnection(id),
        ErpService.getSyncLogs(id),
      ]);

      setConnection(connData);
      setSyncLogs(logsData);
    } catch (error: any) {
      console.error('Failed to load sync logs:', error);
      setSyncLogs([]);
      router.push('/dashboard/erp');
    } finally {
      setLoading(false);
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      const logsData = await ErpService.getSyncLogs(id);
      setSyncLogs(logsData);
      toast.success('Sync logs refreshed');
    } catch (error: any) {
      console.error('Failed to refresh sync logs:', error);
      setSyncLogs([]);
    } finally {
      setRefreshing(false);
    }
  };

  const handleSyncComplete = async () => {
    // Refresh logs after sync completes
    await handleRefresh();
  };

  const handleViewAnalysis = (logId: string) => {
    setSelectedLogId(logId);
    setShowAiAnalysis(true);
  };

  const getFilteredLogs = (): SyncLog[] => {
    if (statusFilter === 'all') {
      return syncLogs;
    }
    return syncLogs.filter(log => log.status === statusFilter);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto mb-3" />
          <p className="text-gray-600 dark:text-gray-400">Loading sync logs...</p>
        </div>
      </div>
    );
  }

  if (!connection) {
    return null;
  }

  const filteredLogs = getFilteredLogs();

  // Calculate stats
  const totalSyncs = syncLogs.length;
  const completedSyncs = syncLogs.filter(log => log.status === 'completed').length;
  const failedSyncs = syncLogs.filter(log => log.status === 'failed').length;
  const partialSyncs = syncLogs.filter(log => log.status === 'partial').length;
  const successRate = totalSyncs > 0 ? ((completedSyncs / totalSyncs) * 100).toFixed(1) : '0';

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Button
          variant="ghost"
          onClick={() => router.push(`/dashboard/erp/${id}`)}
          className="gap-2 mb-4"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Connection
        </Button>

        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              Sync Logs
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              {connection.connection_name}
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={handleRefresh}
              disabled={refreshing}
              className="gap-2"
            >
              <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
              Refresh
            </Button>

            <SyncTriggerButton
              connectionId={id}
              onSyncComplete={handleSyncComplete}
            />
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Total Syncs</span>
            <RefreshCw className="h-5 w-5 text-blue-600" />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {totalSyncs}
          </div>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Success Rate</span>
            <div className={`w-3 h-3 rounded-full ${
              parseFloat(successRate) >= 90 ? 'bg-green-600' :
              parseFloat(successRate) >= 70 ? 'bg-yellow-600' :
              'bg-red-600'
            }`} />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {successRate}%
          </div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">
            {completedSyncs} of {totalSyncs} successful
          </p>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Failed Syncs</span>
            <div className="w-3 h-3 rounded-full bg-red-600" />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {failedSyncs}
          </div>
          {partialSyncs > 0 && (
            <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">
              +{partialSyncs} partial
            </p>
          )}
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Last Sync</span>
            <Calendar className="h-5 w-5 text-purple-600" />
          </div>
          <div className="text-lg font-bold text-gray-900 dark:text-white">
            {syncLogs.length > 0 ? (
              new Date(syncLogs[0].started_at).toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric',
              })
            ) : (
              'Never'
            )}
          </div>
          {syncLogs.length > 0 && (
            <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">
              {new Date(syncLogs[0].started_at).toLocaleTimeString('en-US', {
                hour: 'numeric',
                minute: '2-digit',
              })}
            </p>
          )}
        </Card>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4 mb-6">
        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4 text-gray-600 dark:text-gray-400" />
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Filter by status:
          </span>
        </div>
        <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as StatusFilter)}>
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Statuses ({syncLogs.length})</SelectItem>
            <SelectItem value="completed">Completed ({completedSyncs})</SelectItem>
            <SelectItem value="failed">Failed ({failedSyncs})</SelectItem>
            <SelectItem value="partial">Partial ({partialSyncs})</SelectItem>
            <SelectItem value="in_progress">In Progress</SelectItem>
            <SelectItem value="pending">Pending</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Sync Logs Table */}
      {filteredLogs.length > 0 ? (
        <SyncHistoryTable
          logs={filteredLogs}
          onViewAnalysis={handleViewAnalysis}
        />
      ) : (
        <Card className="p-12 text-center">
          <RefreshCw className="h-16 w-16 text-gray-400 dark:text-gray-600 mx-auto mb-4" />
          <h3 className="text-xl font-bold text-gray-900 dark:text-white mb-2">
            {statusFilter === 'all' ? 'No Sync Logs Yet' : 'No Logs Match Filter'}
          </h3>
          <p className="text-gray-600 dark:text-gray-400 mb-6">
            {statusFilter === 'all'
              ? 'Trigger a sync to start synchronizing inventory between Atlas and your ERP system.'
              : `No sync logs with status "${statusFilter}". Try changing the filter.`
            }
          </p>
          {statusFilter === 'all' && (
            <SyncTriggerButton
              connectionId={id}
              onSyncComplete={handleSyncComplete}
            />
          )}
        </Card>
      )}

      {/* AI Error Analysis Modal */}
      {showAiAnalysis && selectedLogId && (
        <AiErrorAnalysis
          syncLogId={selectedLogId}
          connectionId={id}
          onClose={() => {
            setShowAiAnalysis(false);
            setSelectedLogId(null);
          }}
        />
      )}
    </div>
  );
}
