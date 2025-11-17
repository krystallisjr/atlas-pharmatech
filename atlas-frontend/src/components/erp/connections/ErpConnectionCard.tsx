'use client';

import Link from 'next/link';
import { Plug, ArrowRight, Calendar, RefreshCw } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import type { ErpConnection } from '@/types/erp';
import { ERP_SYSTEMS } from '@/types/erp';

interface ErpConnectionCardProps {
  connection: ErpConnection;
}

export function ErpConnectionCard({ connection }: ErpConnectionCardProps) {
  const systemInfo = ERP_SYSTEMS[connection.erp_type];
  const statusColor = connection.status === 'active' ? 'green' :
                      connection.status === 'error' ? 'red' : 'gray';

  return (
    <Link href={`/dashboard/erp/${connection.id}`}>
      <Card className="p-6 hover:shadow-lg transition-all cursor-pointer border-2 hover:border-blue-400 dark:hover:border-blue-600">
        <div className="flex items-start gap-4">
          {/* System Icon */}
          <div className={`w-14 h-14 rounded-xl bg-${systemInfo.color}-100 dark:bg-${systemInfo.color}-900 flex items-center justify-center flex-shrink-0`}>
            <Plug className={`h-7 w-7 text-${systemInfo.color}-600 dark:text-${systemInfo.color}-400`} />
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            {/* Header */}
            <div className="flex items-start justify-between mb-2">
              <div className="flex-1 min-w-0">
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white truncate">
                  {connection.connection_name}
                </h3>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {systemInfo.name}
                </p>
              </div>

              <Badge
                variant={statusColor === 'green' ? 'default' : 'secondary'}
                className={`ml-2 ${
                  statusColor === 'green' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : ''
                } ${
                  statusColor === 'red' ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' : ''
                }`}
              >
                {connection.status}
              </Badge>
            </div>

            {/* Stats */}
            <div className="grid grid-cols-2 gap-4 mt-4">
              <div className="flex items-center gap-2 text-sm">
                <Calendar className="h-4 w-4 text-gray-400" />
                <div>
                  <p className="text-gray-500 dark:text-gray-400 text-xs">Created</p>
                  <p className="text-gray-900 dark:text-white font-medium">
                    {new Date(connection.created_at).toLocaleDateString('en-US', {
                      month: 'short',
                      day: 'numeric',
                    })}
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-2 text-sm">
                <RefreshCw className="h-4 w-4 text-gray-400" />
                <div>
                  <p className="text-gray-500 dark:text-gray-400 text-xs">Last Sync</p>
                  <p className="text-gray-900 dark:text-white font-medium">
                    {connection.last_sync_at ? (
                      new Date(connection.last_sync_at).toLocaleDateString('en-US', {
                        month: 'short',
                        day: 'numeric',
                      })
                    ) : (
                      'Never'
                    )}
                  </p>
                </div>
              </div>
            </div>

            {/* Sync Status */}
            <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <span className="text-sm text-gray-600 dark:text-gray-400">
                {connection.sync_enabled ? (
                  <>Auto-sync every {connection.sync_frequency_minutes} min</>
                ) : (
                  'Manual sync only'
                )}
              </span>

              <div className="flex items-center gap-1 text-blue-600 dark:text-blue-400 font-medium text-sm">
                View Details
                <ArrowRight className="h-4 w-4" />
              </div>
            </div>
          </div>
        </div>
      </Card>
    </Link>
  );
}
