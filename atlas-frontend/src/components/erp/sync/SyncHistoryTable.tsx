'use client';

import { useState } from 'react';
import { ChevronDown, ChevronUp, Sparkles, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { SyncLog } from '@/types/erp';
import { SyncStatusBadge } from './SyncStatusBadge';

interface SyncHistoryTableProps {
  logs: SyncLog[];
  onViewAnalysis: (logId: string) => void;
}

export function SyncHistoryTable({ logs, onViewAnalysis }: SyncHistoryTableProps) {
  const [expandedLogId, setExpandedLogId] = useState<string | null>(null);

  const toggleExpand = (logId: string) => {
    setExpandedLogId(expandedLogId === logId ? null : logId);
  };

  const formatDuration = (seconds: number | null): string => {
    if (!seconds) return 'N/A';
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  };

  const formatDirection = (direction: string): string => {
    switch (direction) {
      case 'atlas_to_erp':
        return 'Atlas → ERP';
      case 'erp_to_atlas':
        return 'ERP → Atlas';
      case 'bidirectional':
        return 'Bidirectional';
      default:
        return direction;
    }
  };

  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead className="bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-12">

              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Started
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Direction
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Status
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Items
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Errors
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Duration
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
            {logs.map((log) => {
              const isExpanded = expandedLogId === log.id;
              const hasErrors = log.error_count > 0;

              return (
                <>
                  {/* Main Row */}
                  <tr
                    key={log.id}
                    className="hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors cursor-pointer"
                    onClick={() => toggleExpand(log.id)}
                  >
                    {/* Expand Icon */}
                    <td className="px-6 py-4 text-center">
                      {isExpanded ? (
                        <ChevronUp className="h-4 w-4 text-gray-400 mx-auto" />
                      ) : (
                        <ChevronDown className="h-4 w-4 text-gray-400 mx-auto" />
                      )}
                    </td>

                    {/* Started */}
                    <td className="px-6 py-4">
                      <div>
                        <p className="text-sm font-medium text-gray-900 dark:text-white">
                          {new Date(log.started_at).toLocaleDateString('en-US', {
                            month: 'short',
                            day: 'numeric',
                            year: 'numeric',
                          })}
                        </p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">
                          {new Date(log.started_at).toLocaleTimeString('en-US', {
                            hour: 'numeric',
                            minute: '2-digit',
                          })}
                        </p>
                      </div>
                    </td>

                    {/* Direction */}
                    <td className="px-6 py-4 text-center">
                      <span className="text-sm text-gray-700 dark:text-gray-300">
                        {formatDirection(log.direction)}
                      </span>
                    </td>

                    {/* Status */}
                    <td className="px-6 py-4 text-center">
                      <SyncStatusBadge status={log.status} />
                    </td>

                    {/* Items */}
                    <td className="px-6 py-4 text-center">
                      <span className="text-sm font-medium text-gray-900 dark:text-white">
                        {log.items_processed || 0}
                      </span>
                    </td>

                    {/* Errors */}
                    <td className="px-6 py-4 text-center">
                      {hasErrors ? (
                        <span className="inline-flex items-center gap-1 text-sm font-medium text-red-600 dark:text-red-400">
                          <AlertCircle className="h-4 w-4" />
                          {log.error_count}
                        </span>
                      ) : (
                        <span className="text-sm text-gray-500 dark:text-gray-400">
                          0
                        </span>
                      )}
                    </td>

                    {/* Duration */}
                    <td className="px-6 py-4 text-center">
                      <span className="text-sm text-gray-700 dark:text-gray-300">
                        {formatDuration(log.duration_seconds)}
                      </span>
                    </td>

                    {/* Actions */}
                    <td className="px-6 py-4 text-center">
                      {hasErrors && (
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={(e) => {
                            e.stopPropagation();
                            onViewAnalysis(log.id);
                          }}
                          className="gap-2 text-purple-600 hover:text-purple-700 hover:bg-purple-50 dark:hover:bg-purple-900/20"
                        >
                          <Sparkles className="h-4 w-4" />
                          AI Analysis
                        </Button>
                      )}
                    </td>
                  </tr>

                  {/* Expanded Details Row */}
                  {isExpanded && (
                    <tr>
                      <td colSpan={8} className="px-6 py-4 bg-gray-50 dark:bg-gray-800">
                        <div className="space-y-4">
                          {/* Sync Details Grid */}
                          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                            <div>
                              <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                                Sync Log ID
                              </label>
                              <p className="text-sm text-gray-900 dark:text-white font-mono mt-1">
                                {log.id.substring(0, 8)}...
                              </p>
                            </div>

                            {log.completed_at && (
                              <div>
                                <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                                  Completed
                                </label>
                                <p className="text-sm text-gray-900 dark:text-white mt-1">
                                  {new Date(log.completed_at).toLocaleTimeString('en-US', {
                                    hour: 'numeric',
                                    minute: '2-digit',
                                  })}
                                </p>
                              </div>
                            )}

                            {log.items_processed !== null && (
                              <div>
                                <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                                  Items Processed
                                </label>
                                <p className="text-sm text-gray-900 dark:text-white mt-1">
                                  {log.items_processed}
                                </p>
                              </div>
                            )}

                            {log.error_count > 0 && (
                              <div>
                                <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                                  Error Count
                                </label>
                                <p className="text-sm text-red-600 dark:text-red-400 font-semibold mt-1">
                                  {log.error_count}
                                </p>
                              </div>
                            )}
                          </div>

                          {/* Error Message */}
                          {log.error_message && (
                            <div className="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg border border-red-200 dark:border-red-800">
                              <div className="flex items-start gap-2">
                                <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
                                <div className="flex-1">
                                  <h4 className="font-medium text-red-900 dark:text-red-200 mb-1">
                                    Error Message
                                  </h4>
                                  <p className="text-sm text-red-800 dark:text-red-300 font-mono">
                                    {log.error_message}
                                  </p>
                                </div>
                              </div>
                            </div>
                          )}

                          {/* AI Analysis Button (if errors) */}
                          {hasErrors && (
                            <div className="flex justify-end">
                              <Button
                                size="sm"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  onViewAnalysis(log.id);
                                }}
                                className="gap-2 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700"
                              >
                                <Sparkles className="h-4 w-4" />
                                Get AI Analysis & Recommendations
                              </Button>
                            </div>
                          )}

                          {/* Success Message */}
                          {log.status === 'completed' && !hasErrors && (
                            <div className="p-4 bg-green-50 dark:bg-green-900/20 rounded-lg border border-green-200 dark:border-green-800">
                              <p className="text-sm text-green-800 dark:text-green-300">
                                Sync completed successfully with no errors.
                              </p>
                            </div>
                          )}
                        </div>
                      </td>
                    </tr>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
