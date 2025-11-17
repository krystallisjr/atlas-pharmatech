'use client';

import { Check, Clock, Database, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DataConflict } from '@/types/erp';

interface ConflictComparisonViewProps {
  conflict: DataConflict;
  useAiRecommendation: boolean;
  customSelection?: 'atlas' | 'erp';
  onSelectValue?: (source: 'atlas' | 'erp') => void;
}

export function ConflictComparisonView({
  conflict,
  useAiRecommendation,
  customSelection,
  onSelectValue,
}: ConflictComparisonViewProps) {
  const isAtlasRecommended = conflict.ai_recommendation?.recommended_value === 'atlas';
  const isErpRecommended = conflict.ai_recommendation?.recommended_value === 'erp';

  const formatTimestamp = (timestamp: string | null): string => {
    if (!timestamp) return 'Unknown';
    const date = new Date(timestamp);
    return date.toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  };

  const formatValue = (value: any): string => {
    if (value === null || value === undefined) return 'N/A';
    if (typeof value === 'object') return JSON.stringify(value, null, 2);
    return String(value);
  };

  return (
    <div className="p-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Atlas Value */}
        <div
          className={`p-6 rounded-lg border-2 transition-all ${
            useAiRecommendation && isAtlasRecommended
              ? 'bg-purple-50 dark:bg-purple-900/20 border-purple-500 dark:border-purple-600'
              : customSelection === 'atlas'
              ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-600'
              : 'bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700'
          }`}
        >
          {/* Header */}
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center">
                <Database className="h-4 w-4" />
              </div>
              <h5 className="font-semibold text-gray-900 dark:text-white">
                Atlas Value
              </h5>
            </div>

            {useAiRecommendation && isAtlasRecommended && (
              <div className="flex items-center gap-1 text-purple-700 dark:text-purple-300 text-sm font-medium">
                <Check className="h-4 w-4" />
                AI Pick
              </div>
            )}
          </div>

          {/* Value Display */}
          <div className="mb-4">
            <div className="p-4 bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700">
              <pre className="text-sm text-gray-900 dark:text-white font-mono whitespace-pre-wrap break-words">
                {formatValue(conflict.atlas_value)}
              </pre>
            </div>
          </div>

          {/* Metadata */}
          <div className="space-y-2 mb-4">
            {conflict.atlas_last_modified && (
              <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                <Clock className="h-3 w-3" />
                <span>Modified: {formatTimestamp(conflict.atlas_last_modified)}</span>
              </div>
            )}

            {conflict.atlas_modified_by && (
              <div className="text-xs text-gray-600 dark:text-gray-400">
                <span>By: {conflict.atlas_modified_by}</span>
              </div>
            )}
          </div>

          {/* Select Button (Manual Mode) */}
          {!useAiRecommendation && (
            <Button
              size="sm"
              variant={customSelection === 'atlas' ? 'default' : 'outline'}
              onClick={() => onSelectValue?.('atlas')}
              className="w-full gap-2"
            >
              {customSelection === 'atlas' ? (
                <>
                  <Check className="h-4 w-4" />
                  Selected
                </>
              ) : (
                'Select This Value'
              )}
            </Button>
          )}
        </div>

        {/* ERP Value */}
        <div
          className={`p-6 rounded-lg border-2 transition-all ${
            useAiRecommendation && isErpRecommended
              ? 'bg-purple-50 dark:bg-purple-900/20 border-purple-500 dark:border-purple-600'
              : customSelection === 'erp'
              ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-600'
              : 'bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700'
          }`}
        >
          {/* Header */}
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-full bg-indigo-600 text-white flex items-center justify-center">
                <Server className="h-4 w-4" />
              </div>
              <h5 className="font-semibold text-gray-900 dark:text-white">
                ERP Value
              </h5>
            </div>

            {useAiRecommendation && isErpRecommended && (
              <div className="flex items-center gap-1 text-purple-700 dark:text-purple-300 text-sm font-medium">
                <Check className="h-4 w-4" />
                AI Pick
              </div>
            )}
          </div>

          {/* Value Display */}
          <div className="mb-4">
            <div className="p-4 bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700">
              <pre className="text-sm text-gray-900 dark:text-white font-mono whitespace-pre-wrap break-words">
                {formatValue(conflict.erp_value)}
              </pre>
            </div>
          </div>

          {/* Metadata */}
          <div className="space-y-2 mb-4">
            {conflict.erp_last_modified && (
              <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                <Clock className="h-3 w-3" />
                <span>Modified: {formatTimestamp(conflict.erp_last_modified)}</span>
              </div>
            )}

            {conflict.erp_modified_by && (
              <div className="text-xs text-gray-600 dark:text-gray-400">
                <span>By: {conflict.erp_modified_by}</span>
              </div>
            )}
          </div>

          {/* Select Button (Manual Mode) */}
          {!useAiRecommendation && (
            <Button
              size="sm"
              variant={customSelection === 'erp' ? 'default' : 'outline'}
              onClick={() => onSelectValue?.('erp')}
              className="w-full gap-2"
            >
              {customSelection === 'erp' ? (
                <>
                  <Check className="h-4 w-4" />
                  Selected
                </>
              ) : (
                'Select This Value'
              )}
            </Button>
          )}
        </div>
      </div>

      {/* Differences Highlight */}
      {conflict.differences && conflict.differences.length > 0 && (
        <div className="mt-4 p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
          <h6 className="font-medium text-yellow-900 dark:text-yellow-200 mb-2 text-sm">
            Key Differences:
          </h6>
          <ul className="list-disc list-inside space-y-1 text-sm text-yellow-800 dark:text-yellow-300">
            {conflict.differences.map((diff, index) => (
              <li key={index}>{diff}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
