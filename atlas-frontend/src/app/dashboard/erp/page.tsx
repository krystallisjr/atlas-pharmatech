'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { Plug, Plus, ArrowRight, Sparkles, RefreshCw, Loader2 } from 'lucide-react';
import { ErpService } from '@/lib/services';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import type { ErpConnection } from '@/types/erp';
import { ERP_SYSTEMS, getSyncStatusColor } from '@/types/erp';
import { toast } from 'react-toastify';

export default function ErpIntegrationPage() {
  const router = useRouter();
  const [connections, setConnections] = useState<ErpConnection[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const loadConnections = async () => {
    try {
      const data = await ErpService.listConnections();
      setConnections(data);
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to load ERP connections');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    loadConnections();
  }, []);

  const handleRefresh = async () => {
    setRefreshing(true);
    await loadConnections();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto mb-3" />
          <p className="text-gray-600 dark:text-gray-400">Loading ERP connections...</p>
        </div>
      </div>
    );
  }

  // Empty state - no connections yet
  if (connections.length === 0) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex justify-between items-center mb-8">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">ERP Integration</h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              Connect to NetSuite or SAP for real-time inventory synchronization
            </p>
          </div>
        </div>

        {/* Empty State */}
        <div className="max-w-3xl mx-auto">
          <Card className="p-12 text-center bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-gray-800 dark:to-gray-900 border-2 border-dashed border-blue-300 dark:border-blue-700">
            <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-blue-100 dark:bg-blue-900 mb-6">
              <Plug className="h-10 w-10 text-blue-600 dark:text-blue-400" />
            </div>

            <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-3">
              No ERP Connections Yet
            </h2>

            <p className="text-gray-600 dark:text-gray-400 mb-8 max-w-md mx-auto">
              Connect your NetSuite or SAP system to automatically sync inventory, enable AI-powered product mapping, and streamline your operations.
            </p>

            {/* Benefits */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8 text-left">
              <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
                <div className="flex items-center gap-3 mb-3">
                  <RefreshCw className="h-5 w-5 text-blue-600" />
                  <h3 className="font-semibold text-gray-900 dark:text-white">Auto-Sync</h3>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Real-time bidirectional inventory synchronization between Atlas and your ERP
                </p>
              </div>

              <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
                <div className="flex items-center gap-3 mb-3">
                  <Sparkles className="h-5 w-5 text-purple-600" />
                  <h3 className="font-semibold text-gray-900 dark:text-white">AI Mapping</h3>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Claude AI automatically matches your products using NDC codes and product names
                </p>
              </div>

              <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
                <div className="flex items-center gap-3 mb-3">
                  <ArrowRight className="h-5 w-5 text-green-600" />
                  <h3 className="font-semibold text-gray-900 dark:text-white">Smart Conflicts</h3>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  AI-powered conflict resolution analyzes timestamps and transaction history
                </p>
              </div>
            </div>

            {/* CTA Button */}
            <Link href="/dashboard/erp/new">
              <Button size="lg" className="gap-2">
                <Plus className="h-5 w-5" />
                Connect Your First ERP System
              </Button>
            </Link>
          </Card>
        </div>
      </div>
    );
  }

  // Connections List View
  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">ERP Integration</h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Manage your NetSuite and SAP connections
          </p>
        </div>

        <div className="flex gap-3">
          <Button
            variant="outline"
            onClick={handleRefresh}
            disabled={refreshing}
            className="gap-2"
          >
            <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
            Refresh
          </Button>

          <Link href="/dashboard/erp/new">
            <Button className="gap-2">
              <Plus className="h-5 w-5" />
              New Connection
            </Button>
          </Link>
        </div>
      </div>

      {/* Connections Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {connections.map((connection) => {
          const systemInfo = ERP_SYSTEMS[connection.erp_type];
          const statusColor = connection.status === 'active' ? 'green' :
                             connection.status === 'error' ? 'red' : 'gray';

          return (
            <Link href={`/dashboard/erp/${connection.id}`} key={connection.id}>
              <Card className="p-6 hover:shadow-lg transition-all cursor-pointer border-2 hover:border-blue-500 dark:hover:border-blue-400">
                {/* Header */}
                <div className="flex items-start justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div className={`w-12 h-12 rounded-lg bg-${systemInfo.color}-100 dark:bg-${systemInfo.color}-900 flex items-center justify-center`}>
                      <Plug className={`h-6 w-6 text-${systemInfo.color}-600 dark:text-${systemInfo.color}-400`} />
                    </div>
                    <div>
                      <h3 className="font-semibold text-gray-900 dark:text-white">
                        {connection.connection_name}
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {systemInfo.name}
                      </p>
                    </div>
                  </div>

                  <Badge
                    variant={statusColor === 'green' ? 'default' : 'secondary'}
                    className={`
                      ${statusColor === 'green' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : ''}
                      ${statusColor === 'red' ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' : ''}
                    `}
                  >
                    {connection.status}
                  </Badge>
                </div>

                {/* Stats */}
                <div className="space-y-3">
                  {connection.last_sync_at && (
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-600 dark:text-gray-400">Last Sync:</span>
                      <span className="font-medium text-gray-900 dark:text-white">
                        {new Date(connection.last_sync_at).toLocaleString()}
                      </span>
                    </div>
                  )}

                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Sync Direction:</span>
                    <span className="font-medium text-gray-900 dark:text-white capitalize">
                      {connection.default_sync_direction.replace('_', ' → ')}
                    </span>
                  </div>

                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">Auto-Sync:</span>
                    <Badge variant={connection.sync_enabled ? 'default' : 'secondary'}>
                      {connection.sync_enabled ? 'Enabled' : 'Disabled'}
                    </Badge>
                  </div>
                </div>

                {/* Footer Actions */}
                <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600 dark:text-gray-400">
                      Created {new Date(connection.created_at).toLocaleDateString()}
                    </span>
                    <ArrowRight className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                </div>
              </Card>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
