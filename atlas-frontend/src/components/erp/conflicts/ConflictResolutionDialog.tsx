'use client';

import { useState } from 'react';
import { X, Sparkles, AlertTriangle, CheckCircle, Loader2, Zap } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ErpService } from '@/lib/services';
import type { DataConflict, ConflictResolutionRequest } from '@/types/erp';
import { toast } from 'react-toastify';
import { ConflictComparisonView } from './ConflictComparisonView';

interface ConflictResolutionDialogProps {
  isOpen: boolean;
  onClose: () => void;
  connectionId: string;
  conflicts: DataConflict[];
  onResolutionComplete?: () => void;
}

export function ConflictResolutionDialog({
  isOpen,
  onClose,
  connectionId,
  conflicts,
  onResolutionComplete,
}: ConflictResolutionDialogProps) {
  const [resolving, setResolving] = useState(false);
  const [aiAnalyzing, setAiAnalyzing] = useState(false);
  const [customSelections, setCustomSelections] = useState<Record<string, 'atlas' | 'erp'>>({});
  const [useAiRecommendations, setUseAiRecommendations] = useState(true);

  if (!isOpen) return null;

  const handleResolveWithAI = async () => {
    setResolving(true);
    setAiAnalyzing(true);

    try {
      // Build resolution request
      const resolutionRequest: ConflictResolutionRequest = {
        conflicts: conflicts.map(conflict => ({
          field_name: conflict.field_name,
          atlas_value: conflict.atlas_value,
          erp_value: conflict.erp_value,
          resolution_strategy: 'ai_recommended', // Use AI recommendation
        })),
      };

      const response = await ErpService.resolveConflicts(connectionId, resolutionRequest);

      toast.success(
        `${response.resolved_count} conflicts resolved successfully!`,
        { autoClose: 5000 }
      );

      onResolutionComplete?.();
      onClose();
    } catch (error: any) {
      const errorMessage = error.response?.data?.error || 'Failed to resolve conflicts';
      toast.error(errorMessage);
      console.error('Conflict resolution error:', error);
    } finally {
      setResolving(false);
      setAiAnalyzing(false);
    }
  };

  const handleResolveCustom = async () => {
    // Check if all conflicts have selections
    const unselected = conflicts.filter(c => !customSelections[c.field_name]);
    if (unselected.length > 0) {
      toast.warning(`Please select a value for all ${unselected.length} remaining conflicts`);
      return;
    }

    setResolving(true);

    try {
      // Build resolution request with custom selections
      const resolutionRequest: ConflictResolutionRequest = {
        conflicts: conflicts.map(conflict => ({
          field_name: conflict.field_name,
          atlas_value: conflict.atlas_value,
          erp_value: conflict.erp_value,
          resolution_strategy: customSelections[conflict.field_name] === 'atlas' ? 'prefer_atlas' : 'prefer_erp',
        })),
      };

      const response = await ErpService.resolveConflicts(connectionId, resolutionRequest);

      toast.success(
        `${response.resolved_count} conflicts resolved with custom selections!`,
        { autoClose: 5000 }
      );

      onResolutionComplete?.();
      onClose();
    } catch (error: any) {
      const errorMessage = error.response?.data?.error || 'Failed to resolve conflicts';
      toast.error(errorMessage);
      console.error('Conflict resolution error:', error);
    } finally {
      setResolving(false);
    }
  };

  const toggleCustomSelection = (fieldName: string, source: 'atlas' | 'erp') => {
    setCustomSelections(prev => ({
      ...prev,
      [fieldName]: source,
    }));
  };

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case 'critical':
        return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
      case 'high':
        return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
      case 'low':
        return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
    }
  };

  const getRiskIcon = (risk: string) => {
    switch (risk) {
      case 'critical':
      case 'high':
        return <AlertTriangle className="h-4 w-4" />;
      case 'medium':
        return <Zap className="h-4 w-4" />;
      case 'low':
        return <CheckCircle className="h-4 w-4" />;
      default:
        return null;
    }
  };

  // Count conflicts by risk level
  const criticalCount = conflicts.filter(c => c.ai_recommendation?.risk_level === 'critical').length;
  const highCount = conflicts.filter(c => c.ai_recommendation?.risk_level === 'high').length;
  const mediumCount = conflicts.filter(c => c.ai_recommendation?.risk_level === 'medium').length;
  const lowCount = conflicts.filter(c => c.ai_recommendation?.risk_level === 'low').length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-2xl max-w-6xl w-full max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="sticky top-0 bg-gradient-to-r from-orange-600 to-red-600 text-white p-6 flex items-center justify-between z-10 rounded-t-lg">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-white/20 flex items-center justify-center">
              <AlertTriangle className="h-6 w-6" />
            </div>
            <div>
              <h2 className="text-2xl font-bold">
                Conflict Resolution
              </h2>
              <p className="text-orange-100 text-sm">
                {conflicts.length} {conflicts.length === 1 ? 'conflict' : 'conflicts'} detected • AI-Powered Resolution
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            disabled={resolving}
            className="text-white hover:bg-white/20"
          >
            <X className="h-5 w-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="p-6">
          {/* Risk Summary */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
            {criticalCount > 0 && (
              <div className="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg border-2 border-red-300 dark:border-red-700">
                <div className="flex items-center gap-2 mb-1">
                  <AlertTriangle className="h-5 w-5 text-red-600" />
                  <span className="text-xs font-medium text-red-900 dark:text-red-200 uppercase">Critical</span>
                </div>
                <p className="text-2xl font-bold text-red-600 dark:text-red-400">{criticalCount}</p>
              </div>
            )}

            {highCount > 0 && (
              <div className="p-4 bg-orange-50 dark:bg-orange-900/20 rounded-lg border border-orange-200 dark:border-orange-800">
                <div className="flex items-center gap-2 mb-1">
                  <AlertTriangle className="h-5 w-5 text-orange-600" />
                  <span className="text-xs font-medium text-orange-900 dark:text-orange-200 uppercase">High Risk</span>
                </div>
                <p className="text-2xl font-bold text-orange-600 dark:text-orange-400">{highCount}</p>
              </div>
            )}

            {mediumCount > 0 && (
              <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
                <div className="flex items-center gap-2 mb-1">
                  <Zap className="h-5 w-5 text-yellow-600" />
                  <span className="text-xs font-medium text-yellow-900 dark:text-yellow-200 uppercase">Medium</span>
                </div>
                <p className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">{mediumCount}</p>
              </div>
            )}

            {lowCount > 0 && (
              <div className="p-4 bg-green-50 dark:bg-green-900/20 rounded-lg border border-green-200 dark:border-green-800">
                <div className="flex items-center gap-2 mb-1">
                  <CheckCircle className="h-5 w-5 text-green-600" />
                  <span className="text-xs font-medium text-green-900 dark:text-green-200 uppercase">Low Risk</span>
                </div>
                <p className="text-2xl font-bold text-green-600 dark:text-green-400">{lowCount}</p>
              </div>
            )}
          </div>

          {/* Mode Toggle */}
          <div className="flex items-center justify-between mb-6 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-1">
                Resolution Mode
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                {useAiRecommendations
                  ? 'AI will automatically select the best value for each conflict'
                  : 'Manually select which value to keep for each conflict'
                }
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setUseAiRecommendations(!useAiRecommendations)}
              disabled={resolving}
            >
              {useAiRecommendations ? 'Switch to Manual' : 'Use AI Recommendations'}
            </Button>
          </div>

          {/* Conflicts List */}
          <div className="space-y-4">
            {conflicts.map((conflict, index) => (
              <div
                key={conflict.field_name}
                className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden"
              >
                {/* Conflict Header */}
                <div className="p-4 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className="w-6 h-6 rounded-full bg-gradient-to-br from-orange-600 to-red-600 text-white flex items-center justify-center text-xs font-bold">
                        {index + 1}
                      </span>
                      <h4 className="font-semibold text-gray-900 dark:text-white">
                        {conflict.field_name.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
                      </h4>
                    </div>

                    {conflict.ai_recommendation && (
                      <Badge className={getRiskColor(conflict.ai_recommendation.risk_level)}>
                        <span className="flex items-center gap-1">
                          {getRiskIcon(conflict.ai_recommendation.risk_level)}
                          {conflict.ai_recommendation.risk_level.toUpperCase()} RISK
                        </span>
                      </Badge>
                    )}
                  </div>

                  {/* AI Recommendation */}
                  {conflict.ai_recommendation && (
                    <div className="mt-3 p-3 bg-purple-50 dark:bg-purple-900/20 rounded border border-purple-200 dark:border-purple-800">
                      <div className="flex items-start gap-2">
                        <Sparkles className="h-4 w-4 text-purple-600 dark:text-purple-400 flex-shrink-0 mt-0.5" />
                        <div className="flex-1">
                          <p className="text-sm text-gray-700 dark:text-gray-300 italic">
                            "{conflict.ai_recommendation.reasoning}"
                          </p>
                          <p className="text-xs text-purple-700 dark:text-purple-300 mt-1 font-medium">
                            AI Recommends: {conflict.ai_recommendation.recommended_value === 'atlas' ? 'Keep Atlas Value' : 'Keep ERP Value'}
                          </p>
                        </div>
                      </div>
                    </div>
                  )}
                </div>

                {/* Conflict Comparison */}
                <ConflictComparisonView
                  conflict={conflict}
                  useAiRecommendation={useAiRecommendations}
                  customSelection={customSelections[conflict.field_name]}
                  onSelectValue={(source) => toggleCustomSelection(conflict.field_name, source)}
                />
              </div>
            ))}
          </div>
        </div>

        {/* Footer Actions */}
        <div className="sticky bottom-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 p-6 flex items-center justify-between rounded-b-lg">
          <div className="text-sm text-gray-600 dark:text-gray-400">
            {conflicts.length} {conflicts.length === 1 ? 'conflict' : 'conflicts'} to resolve
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              onClick={onClose}
              disabled={resolving}
            >
              Cancel
            </Button>

            {useAiRecommendations ? (
              <Button
                onClick={handleResolveWithAI}
                disabled={resolving || aiAnalyzing}
                className="gap-2 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700"
              >
                {resolving ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {aiAnalyzing ? 'AI Analyzing...' : 'Resolving...'}
                  </>
                ) : (
                  <>
                    <Sparkles className="h-4 w-4" />
                    Accept AI Recommendations
                  </>
                )}
              </Button>
            ) : (
              <Button
                onClick={handleResolveCustom}
                disabled={resolving || Object.keys(customSelections).length !== conflicts.length}
                className="gap-2 bg-gradient-to-r from-orange-600 to-red-600 hover:from-orange-700 hover:to-red-700"
              >
                {resolving ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Resolving...
                  </>
                ) : (
                  <>
                    <CheckCircle className="h-4 w-4" />
                    Resolve with Custom Selections ({Object.keys(customSelections).length}/{conflicts.length})
                  </>
                )}
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
