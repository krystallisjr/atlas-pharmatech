'use client';

import { CheckCircle, Database, Sparkles } from 'lucide-react';
import { Card } from '@/components/ui/card';

interface MappingStatusIndicatorProps {
  mapped: number;
  total: number;
  percentage: number;
  suggested: number;
}

export function MappingStatusIndicator({
  mapped,
  total,
  percentage,
  suggested,
}: MappingStatusIndicatorProps) {
  // Determine progress color
  const getProgressColor = () => {
    if (percentage >= 90) return 'bg-green-600';
    if (percentage >= 70) return 'bg-blue-600';
    if (percentage >= 40) return 'bg-yellow-600';
    return 'bg-gray-600';
  };

  const getProgressGradient = () => {
    if (percentage >= 90) return 'from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20';
    if (percentage >= 70) return 'from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20';
    if (percentage >= 40) return 'from-yellow-50 to-amber-50 dark:from-yellow-900/20 dark:to-amber-900/20';
    return 'from-gray-50 to-slate-50 dark:from-gray-800 dark:to-slate-800';
  };

  return (
    <Card className={`p-8 mb-8 bg-gradient-to-br ${getProgressGradient()} border-2`}>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
        {/* Progress Bar Section */}
        <div className="md:col-span-2">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              Mapping Progress
            </h3>
            <span className="text-3xl font-bold text-gray-900 dark:text-white">
              {percentage.toFixed(0)}%
            </span>
          </div>

          {/* Progress Bar */}
          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-4 mb-3 overflow-hidden">
            <div
              className={`h-4 ${getProgressColor()} transition-all duration-1000 ease-out rounded-full`}
              style={{ width: `${Math.min(percentage, 100)}%` }}
            >
              <div className="h-full w-full bg-gradient-to-r from-transparent via-white/20 to-transparent animate-shimmer" />
            </div>
          </div>

          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-600 dark:text-gray-400">
              {mapped} of {total} products mapped
            </span>
            <span className="text-gray-600 dark:text-gray-400">
              {total - mapped} remaining
            </span>
          </div>
        </div>

        {/* Stats Section */}
        <div className="flex flex-col gap-4">
          {/* Mapped Count */}
          <div className="flex items-center gap-3 p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <div className="flex items-center justify-center w-10 h-10 rounded-full bg-green-100 dark:bg-green-900">
              <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400" />
            </div>
            <div>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {mapped}
              </p>
              <p className="text-xs text-gray-600 dark:text-gray-400">
                Mapped Products
              </p>
            </div>
          </div>

          {/* AI Suggestions Count */}
          {suggested > 0 && (
            <div className="flex items-center gap-3 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg border border-purple-200 dark:border-purple-800">
              <div className="flex items-center justify-center w-10 h-10 rounded-full bg-purple-100 dark:bg-purple-900">
                <Sparkles className="h-5 w-5 text-purple-600 dark:text-purple-400" />
              </div>
              <div>
                <p className="text-2xl font-bold text-gray-900 dark:text-white">
                  {suggested}
                </p>
                <p className="text-xs text-purple-700 dark:text-purple-300 font-medium">
                  AI Suggestions
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Completion Message */}
      {percentage >= 100 && (
        <div className="mt-6 p-4 bg-green-100 dark:bg-green-900/30 rounded-lg border border-green-300 dark:border-green-700">
          <div className="flex items-center gap-2">
            <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400" />
            <p className="text-sm font-medium text-green-800 dark:text-green-200">
              All products are mapped! Your inventory is fully synchronized with your ERP system.
            </p>
          </div>
        </div>
      )}

      {/* Low Progress Warning */}
      {percentage < 40 && total > 0 && (
        <div className="mt-6 p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
            <p className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
              Use AI Auto-Discovery to quickly find and map your inventory items.
            </p>
          </div>
        </div>
      )}
    </Card>
  );
}
