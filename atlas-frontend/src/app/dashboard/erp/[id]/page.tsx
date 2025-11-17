'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { ArrowLeft, Plug, RefreshCw, Trash2, Loader2, Sparkles, Database, History } from 'lucide-react';
import { ErpService } from '@/lib/services';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { ErpConnection, MappingStatus } from '@/types/erp';
import { ERP_SYSTEMS } from '@/types/erp';
import { toast } from 'react-toastify';

export default function ErpConnectionDetailsPage({ params }: { params: { id: string } }) {
  const router = useRouter();
  const [connection, setConnection] = useState<ErpConnection | null>(null);
  const [mappingStatus, setMappingStatus] = useState<MappingStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    loadConnection();
    loadMappingStatus();
  }, [params.id]);

  const loadConnection = async () => {
    try {
      const data = await ErpService.getConnection(params.id);
      setConnection(data);
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to load connection');
      router.push('/dashboard/erp');
    } finally {
      setLoading(false);
    }
  };

  const loadMappingStatus = async () => {
    try {
      const status = await ErpService.getMappingStatus(params.id);
      setMappingStatus(status);
    } catch (error: any) {
      console.error('Failed to load mapping status:', error);
    }
  };

  const handleDelete = async () => {
    if (!confirm(`Are you sure you want to delete this connection? This action cannot be undone.`)) {
      return;
    }

    setDeleting(true);
    try {
      await ErpService.deleteConnection(params.id);
      toast.success('Connection deleted successfully');
      router.push('/dashboard/erp');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to delete connection');
      setDeleting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto mb-3" />
          <p className="text-gray-600 dark:text-gray-400">Loading connection...</p>
        </div>
      </div>
    );
  }

  if (!connection) {
    return null;
  }

  const systemInfo = ERP_SYSTEMS[connection.erp_type];
  const statusColor = connection.status === 'active' ? 'green' :
                      connection.status === 'error' ? 'red' : 'gray';

  const mappingPercentage = mappingStatus?.mapping_percentage || 0;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Button
          variant="ghost"
          onClick={() => router.push('/dashboard/erp')}
          className="gap-2 mb-4"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to ERP Integration
        </Button>

        <div className="flex items-start justify-between">
          <div className="flex items-center gap-4">
            <div className={`w-16 h-16 rounded-xl bg-${systemInfo.color}-100 dark:bg-${systemInfo.color}-900 flex items-center justify-center`}>
              <Plug className={`h-8 w-8 text-${systemInfo.color}-600 dark:text-${systemInfo.color}-400`} />
            </div>

            <div>
              <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
                {connection.connection_name}
              </h1>
              <p className="text-gray-600 dark:text-gray-400 mt-1">
                {systemInfo.name} • Created {new Date(connection.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Badge
              variant={statusColor === 'green' ? 'default' : 'secondary'}
              className={`
                ${statusColor === 'green' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : ''}
                ${statusColor === 'red' ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' : ''}
              `}
            >
              {connection.status}
            </Badge>

            <Button
              variant="destructive"
              size="sm"
              onClick={handleDelete}
              disabled={deleting}
              className="gap-2"
            >
              <Trash2 className="h-4 w-4" />
              {deleting ? 'Deleting...' : 'Delete'}
            </Button>
          </div>
        </div>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Mapping Progress</span>
            <Sparkles className="h-5 w-5 text-purple-600" />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {mappingPercentage.toFixed(0)}%
          </div>
          <div className="mt-2 w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div
              className="bg-purple-600 h-2 rounded-full transition-all"
              style={{ width: `${mappingPercentage}%` }}
            />
          </div>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Mapped Items</span>
            <Database className="h-5 w-5 text-blue-600" />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {mappingStatus?.mapped_count || 0}
          </div>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            of {mappingStatus?.total_atlas_items || 0} products
          </p>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Last Sync</span>
            <RefreshCw className="h-5 w-5 text-green-600" />
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {connection.last_sync_at ? (
              new Date(connection.last_sync_at).toLocaleDateString()
            ) : (
              'Never'
            )}
          </div>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            {connection.sync_enabled ? 'Auto-sync enabled' : 'Manual sync'}
          </p>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Sync Direction</span>
            <History className="h-5 w-5 text-indigo-600" />
          </div>
          <div className="text-lg font-bold text-gray-900 dark:text-white capitalize">
            {connection.default_sync_direction.replace('_', ' → ')}
          </div>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            {connection.conflict_resolution}
          </p>
        </Card>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="overview" className="space-y-6">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="mappings">
            Mappings
            {mappingStatus && mappingStatus.suggested_count > 0 && (
              <Badge className="ml-2 bg-purple-600">
                {mappingStatus.suggested_count}
              </Badge>
            )}
          </TabsTrigger>
          <TabsTrigger value="sync">Sync Logs</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-6">
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Connection Details
            </h3>

            <div className="grid grid-cols-2 gap-6">
              <div>
                <label className="text-sm text-gray-600 dark:text-gray-400">ERP Type</label>
                <p className="font-medium text-gray-900 dark:text-white mt-1">
                  {systemInfo.name}
                </p>
              </div>

              <div>
                <label className="text-sm text-gray-600 dark:text-gray-400">Status</label>
                <p className="font-medium text-gray-900 dark:text-white mt-1 capitalize">
                  {connection.status}
                </p>
              </div>

              <div>
                <label className="text-sm text-gray-600 dark:text-gray-400">Sync Frequency</label>
                <p className="font-medium text-gray-900 dark:text-white mt-1">
                  Every {connection.sync_frequency_minutes} minutes
                </p>
              </div>

              <div>
                <label className="text-sm text-gray-600 dark:text-gray-400">Created</label>
                <p className="font-medium text-gray-900 dark:text-white mt-1">
                  {new Date(connection.created_at).toLocaleString()}
                </p>
              </div>
            </div>
          </Card>

          {/* Quick Actions */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <Link href={`/dashboard/erp/${connection.id}/mappings`}>
              <Card className="p-6 hover:shadow-lg transition-all cursor-pointer border-2 hover:border-purple-500">
                <Sparkles className="h-8 w-8 text-purple-600 mb-3" />
                <h4 className="font-semibold text-gray-900 dark:text-white mb-2">
                  AI Auto-Discovery
                </h4>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Let AI match your inventory with {systemInfo.name}
                </p>
              </Card>
            </Link>

            <Card className="p-6 hover:shadow-lg transition-all cursor-pointer border-2 hover:border-blue-500">
              <RefreshCw className="h-8 w-8 text-blue-600 mb-3" />
              <h4 className="font-semibold text-gray-900 dark:text-white mb-2">
                Trigger Sync
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Manually sync inventory between systems
              </p>
            </Card>

            <Card className="p-6 hover:shadow-lg transition-all cursor-pointer border-2 hover:border-green-500">
              <History className="h-8 w-8 text-green-600 mb-3" />
              <h4 className="font-semibold text-gray-900 dark:text-white mb-2">
                View Sync Logs
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                See sync history and AI error analysis
              </p>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="mappings">
          <Card className="p-6">
            <p className="text-gray-600 dark:text-gray-400">
              Navigate to the <Link href={`/dashboard/erp/${connection.id}/mappings`} className="text-blue-600 hover:underline">Mappings page</Link> to manage inventory mappings.
            </p>
          </Card>
        </TabsContent>

        <TabsContent value="sync">
          <Card className="p-6">
            <p className="text-gray-600 dark:text-gray-400">
              Sync logs feature coming soon...
            </p>
          </Card>
        </TabsContent>

        <TabsContent value="settings">
          <Card className="p-6">
            <p className="text-gray-600 dark:text-gray-400">
              Connection settings coming soon...
            </p>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
