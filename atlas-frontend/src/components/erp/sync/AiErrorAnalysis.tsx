'use client';

import { useEffect, useState } from 'react';
import { X, Sparkles, Lightbulb, AlertTriangle, CheckCircle, Copy, Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ErpService } from '@/lib/services';
import type { SyncInsight } from '@/types/erp';
import { toast } from 'react-toastify';

interface AiErrorAnalysisProps {
  syncLogId: string;
  connectionId: string;
  onClose: () => void;
}

export function AiErrorAnalysis({ syncLogId, connectionId, onClose }: AiErrorAnalysisProps) {
  const [insight, setInsight] = useState<SyncInsight | null>(null);
  const [loading, setLoading] = useState(true);
  const [retrying, setRetrying] = useState(false);

  useEffect(() => {
    loadAnalysis();
  }, [syncLogId]);

  const loadAnalysis = async () => {
    try {
      setLoading(true);
      const data = await ErpService.getSyncAnalysis(syncLogId);
      setInsight(data);
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to load AI analysis');
      console.error('Analysis error:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = () => {
    if (!insight) return;

    const text = `
AI Sync Analysis
================

Summary: ${insight.summary}

Root Cause: ${insight.root_cause}

Recommendations:
${insight.recommendations.map((rec, i) => `${i + 1}. [${rec.priority.toUpperCase()}] ${rec.action}`).join('\n')}

Error Details:
${insight.error_details}
    `.trim();

    navigator.clipboard.writeText(text);
    toast.success('Analysis copied to clipboard');
  };

  const handleRetrySyncWithFixes = async () => {
    setRetrying(true);
    try {
      // Trigger a new sync
      await ErpService.triggerSync(connectionId);
      toast.success('Sync retry initiated');
      onClose();
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to retry sync');
    } finally {
      setRetrying(false);
    }
  };

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case 'high':
        return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
      case 'low':
        return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
    }
  };

  const getPriorityIcon = (priority: string) => {
    switch (priority) {
      case 'high':
        return <AlertTriangle className="h-4 w-4" />;
      case 'medium':
        return <Lightbulb className="h-4 w-4" />;
      case 'low':
        return <CheckCircle className="h-4 w-4" />;
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="sticky top-0 bg-gradient-to-r from-purple-600 to-indigo-600 text-white p-6 flex items-center justify-between z-10 rounded-t-lg">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-white/20 flex items-center justify-center">
              <Sparkles className="h-6 w-6" />
            </div>
            <div>
              <h2 className="text-2xl font-bold">
                AI Error Analysis
              </h2>
              <p className="text-purple-100 text-sm">
                Powered by Claude AI
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="text-white hover:bg-white/20"
          >
            <X className="h-5 w-5" />
          </Button>
        </div>

        {/* Content */}
        {loading ? (
          <div className="flex items-center justify-center py-16">
            <div className="text-center">
              <Loader2 className="h-12 w-12 animate-spin text-purple-600 mx-auto mb-4" />
              <p className="text-gray-600 dark:text-gray-400">
                AI is analyzing the sync error...
              </p>
            </div>
          </div>
        ) : insight ? (
          <div className="p-6 space-y-6">
            {/* Summary Section */}
            <div className="p-6 bg-gradient-to-br from-purple-50 to-indigo-50 dark:from-purple-900/20 dark:to-indigo-900/20 rounded-lg border-2 border-purple-200 dark:border-purple-800">
              <div className="flex items-start gap-3">
                <Sparkles className="h-6 w-6 text-purple-600 dark:text-purple-400 flex-shrink-0 mt-1" />
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                    What Happened (Plain English)
                  </h3>
                  <p className="text-gray-700 dark:text-gray-300 leading-relaxed">
                    {insight.summary}
                  </p>
                </div>
              </div>
            </div>

            {/* Root Cause Section */}
            <div className="p-6 bg-red-50 dark:bg-red-900/20 rounded-lg border border-red-200 dark:border-red-800">
              <div className="flex items-start gap-3">
                <AlertTriangle className="h-6 w-6 text-red-600 dark:text-red-400 flex-shrink-0 mt-1" />
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                    Root Cause
                  </h3>
                  <p className="text-gray-700 dark:text-gray-300">
                    {insight.root_cause}
                  </p>
                </div>
              </div>
            </div>

            {/* Recommendations Section */}
            <div>
              <div className="flex items-center gap-2 mb-4">
                <Lightbulb className="h-6 w-6 text-yellow-600 dark:text-yellow-400" />
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                  How to Fix It
                </h3>
              </div>

              <div className="space-y-3">
                {insight.recommendations.map((rec, index) => (
                  <div
                    key={index}
                    className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 hover:shadow-md transition-shadow"
                  >
                    <div className="flex items-start gap-3">
                      <div className="flex-shrink-0">
                        <div className={`w-8 h-8 rounded-full bg-gradient-to-br from-purple-600 to-indigo-600 text-white flex items-center justify-center font-bold text-sm`}>
                          {index + 1}
                        </div>
                      </div>

                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-2">
                          <Badge className={getPriorityColor(rec.priority)}>
                            <span className="flex items-center gap-1">
                              {getPriorityIcon(rec.priority)}
                              {rec.priority.toUpperCase()} PRIORITY
                            </span>
                          </Badge>
                        </div>

                        <p className="text-gray-900 dark:text-white font-medium mb-2">
                          {rec.action}
                        </p>

                        {rec.details && (
                          <p className="text-sm text-gray-600 dark:text-gray-400">
                            {rec.details}
                          </p>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Technical Error Details (Collapsible) */}
            {insight.error_details && (
              <details className="group">
                <summary className="cursor-pointer p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors">
                  <span className="font-medium text-gray-900 dark:text-white">
                    View Technical Error Details
                  </span>
                </summary>
                <div className="mt-3 p-4 bg-gray-900 dark:bg-gray-950 rounded-lg">
                  <pre className="text-xs text-gray-300 dark:text-gray-400 font-mono whitespace-pre-wrap break-words">
                    {insight.error_details}
                  </pre>
                </div>
              </details>
            )}

            {/* AI Confidence */}
            {insight.confidence_score !== undefined && (
              <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    AI Analysis Confidence
                  </span>
                  <div className="flex items-center gap-2">
                    <div className="w-32 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-blue-600 to-purple-600"
                        style={{ width: `${insight.confidence_score * 100}%` }}
                      />
                    </div>
                    <span className="text-sm font-bold text-gray-900 dark:text-white">
                      {(insight.confidence_score * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="p-12 text-center">
            <AlertTriangle className="h-16 w-16 text-gray-400 dark:text-gray-600 mx-auto mb-4" />
            <p className="text-gray-600 dark:text-gray-400">
              No AI analysis available for this sync log.
            </p>
          </div>
        )}

        {/* Footer Actions */}
        <div className="sticky bottom-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 p-6 flex items-center justify-between rounded-b-lg">
          <Button
            variant="outline"
            onClick={handleCopy}
            disabled={!insight}
            className="gap-2"
          >
            <Copy className="h-4 w-4" />
            Copy Analysis
          </Button>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              onClick={onClose}
            >
              Close
            </Button>

            <Button
              onClick={handleRetrySyncWithFixes}
              disabled={retrying || !insight}
              className="gap-2 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700"
            >
              {retrying ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4" />
              )}
              Retry Sync
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
