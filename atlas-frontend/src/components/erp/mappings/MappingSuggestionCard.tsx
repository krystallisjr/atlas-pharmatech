'use client';

import { useState } from 'react';
import { Check, X, ArrowRight, ChevronDown, ChevronUp, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import type { MappingSuggestion } from '@/types/erp';
import { getConfidenceColor, getConfidenceLabel } from '@/types/erp';
import { MappingReviewDialog } from './MappingReviewDialog';

interface MappingSuggestionCardProps {
  suggestion: MappingSuggestion;
  onApprove: () => Promise<void>;
  onReject: () => Promise<void>;
  bulkSelecting?: boolean;
  isSelected?: boolean;
  onToggleSelect?: () => void;
}

export function MappingSuggestionCard({
  suggestion,
  onApprove,
  onReject,
  bulkSelecting = false,
  isSelected = false,
  onToggleSelect,
}: MappingSuggestionCardProps) {
  const [approving, setApproving] = useState(false);
  const [rejecting, setRejecting] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [showReviewDialog, setShowReviewDialog] = useState(false);

  const confidenceColor = getConfidenceColor(suggestion.confidence_score);
  const confidenceLabel = getConfidenceLabel(suggestion.confidence_score);

  const handleApprove = async () => {
    setApproving(true);
    try {
      await onApprove();
    } finally {
      setApproving(false);
    }
  };

  const handleReject = async () => {
    setRejecting(true);
    try {
      await onReject();
    } finally {
      setRejecting(false);
    }
  };

  const confidenceBadgeClass = {
    green: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    red: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
  }[confidenceColor];

  return (
    <>
      <Card
        className={`p-6 transition-all ${
          bulkSelecting
            ? isSelected
              ? 'border-2 border-blue-500 bg-blue-50 dark:bg-blue-900/20'
              : 'border-2 border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-700 cursor-pointer'
            : 'border hover:shadow-lg'
        }`}
        onClick={() => bulkSelecting && onToggleSelect?.()}
      >
        <div className="flex items-start gap-6">
          {/* Checkbox for bulk selection */}
          {bulkSelecting && (
            <div className="flex items-center pt-1">
              <input
                type="checkbox"
                checked={isSelected}
                onChange={onToggleSelect}
                className="w-5 h-5 text-blue-600 rounded focus:ring-blue-500"
              />
            </div>
          )}

          {/* Atlas Product */}
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                Atlas Product
              </span>
            </div>
            <h3 className="font-semibold text-gray-900 dark:text-white text-lg mb-1">
              {suggestion.atlas_product.product_name}
            </h3>
            <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
              {suggestion.atlas_product.ndc && (
                <p>
                  <span className="font-medium">NDC:</span> {suggestion.atlas_product.ndc}
                </p>
              )}
              {suggestion.atlas_product.manufacturer && (
                <p>
                  <span className="font-medium">Manufacturer:</span> {suggestion.atlas_product.manufacturer}
                </p>
              )}
              {suggestion.atlas_product.strength && (
                <p>
                  <span className="font-medium">Strength:</span> {suggestion.atlas_product.strength}
                </p>
              )}
            </div>
          </div>

          {/* Mapping Arrow */}
          <div className="flex items-center pt-6">
            <ArrowRight className="h-8 w-8 text-purple-600 dark:text-purple-400" />
          </div>

          {/* ERP Product */}
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                {suggestion.erp_product.system} Product
              </span>
            </div>
            <h3 className="font-semibold text-gray-900 dark:text-white text-lg mb-1">
              {suggestion.erp_product.item_name || suggestion.erp_product.item_id}
            </h3>
            <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
              <p>
                <span className="font-medium">Item ID:</span> {suggestion.erp_product.item_id}
              </p>
              {suggestion.erp_product.description && (
                <p>
                  <span className="font-medium">Description:</span> {suggestion.erp_product.description}
                </p>
              )}
              {suggestion.erp_product.manufacturer && (
                <p>
                  <span className="font-medium">Manufacturer:</span> {suggestion.erp_product.manufacturer}
                </p>
              )}
            </div>
          </div>

          {/* Confidence Score & Actions */}
          <div className="flex flex-col items-end gap-3 min-w-[180px]">
            <div className="text-right">
              <Badge className={confidenceBadgeClass}>
                {confidenceLabel}
              </Badge>
              <p className="text-2xl font-bold text-gray-900 dark:text-white mt-2">
                {(suggestion.confidence_score * 100).toFixed(0)}%
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Confidence
              </p>
            </div>

            {!bulkSelecting && (
              <div className="flex flex-col gap-2 w-full">
                <Button
                  size="sm"
                  onClick={handleApprove}
                  disabled={approving || rejecting}
                  className="gap-2 bg-green-600 hover:bg-green-700 w-full"
                >
                  {approving ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Check className="h-4 w-4" />
                  )}
                  Approve
                </Button>

                <Button
                  size="sm"
                  variant="destructive"
                  onClick={handleReject}
                  disabled={approving || rejecting}
                  className="gap-2 w-full"
                >
                  {rejecting ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <X className="h-4 w-4" />
                  )}
                  Reject
                </Button>

                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setShowReviewDialog(true)}
                  className="w-full"
                >
                  View Details
                </Button>
              </div>
            )}
          </div>
        </div>

        {/* AI Reasoning */}
        <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
          <div className="flex items-start gap-2">
            <div className="flex-1">
              <p className="text-sm text-gray-700 dark:text-gray-300 italic">
                "{suggestion.ai_reasoning}"
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setExpanded(!expanded)}
              className="gap-1"
            >
              {expanded ? (
                <>
                  <ChevronUp className="h-4 w-4" />
                  Less
                </>
              ) : (
                <>
                  <ChevronDown className="h-4 w-4" />
                  More
                </>
              )}
            </Button>
          </div>

          {/* Matching Factors (Expanded) */}
          {expanded && (
            <div className="mt-4 grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <h4 className="font-medium text-sm text-gray-900 dark:text-white">
                  Matching Factors:
                </h4>
                <div className="space-y-1">
                  {suggestion.matching_factors.ndc_match !== undefined && (
                    <div className="flex items-center gap-2 text-sm">
                      {suggestion.matching_factors.ndc_match ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-gray-400" />
                      )}
                      <span className={suggestion.matching_factors.ndc_match ? 'text-green-700 dark:text-green-400' : 'text-gray-600 dark:text-gray-400'}>
                        NDC Match
                      </span>
                    </div>
                  )}

                  {suggestion.matching_factors.name_similarity !== undefined && (
                    <div className="flex items-center gap-2 text-sm">
                      <div className={`w-4 h-4 rounded-full ${
                        suggestion.matching_factors.name_similarity >= 0.8 ? 'bg-green-600' :
                        suggestion.matching_factors.name_similarity >= 0.6 ? 'bg-yellow-600' :
                        'bg-gray-400'
                      }`} />
                      <span className="text-gray-700 dark:text-gray-300">
                        Name Similarity: {(suggestion.matching_factors.name_similarity * 100).toFixed(0)}%
                      </span>
                    </div>
                  )}

                  {suggestion.matching_factors.manufacturer_match !== undefined && (
                    <div className="flex items-center gap-2 text-sm">
                      {suggestion.matching_factors.manufacturer_match ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-gray-400" />
                      )}
                      <span className={suggestion.matching_factors.manufacturer_match ? 'text-green-700 dark:text-green-400' : 'text-gray-600 dark:text-gray-400'}>
                        Manufacturer Match
                      </span>
                    </div>
                  )}

                  {suggestion.matching_factors.strength_match !== undefined && (
                    <div className="flex items-center gap-2 text-sm">
                      {suggestion.matching_factors.strength_match ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-gray-400" />
                      )}
                      <span className={suggestion.matching_factors.strength_match ? 'text-green-700 dark:text-green-400' : 'text-gray-600 dark:text-gray-400'}>
                        Strength Match
                      </span>
                    </div>
                  )}

                  {suggestion.matching_factors.dosage_form_match !== undefined && (
                    <div className="flex items-center gap-2 text-sm">
                      {suggestion.matching_factors.dosage_form_match ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-gray-400" />
                      )}
                      <span className={suggestion.matching_factors.dosage_form_match ? 'text-green-700 dark:text-green-400' : 'text-gray-600 dark:text-gray-400'}>
                        Dosage Form Match
                      </span>
                    </div>
                  )}
                </div>
              </div>

              {suggestion.matching_factors.additional_notes && (
                <div>
                  <h4 className="font-medium text-sm text-gray-900 dark:text-white mb-2">
                    Additional Notes:
                  </h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {suggestion.matching_factors.additional_notes}
                  </p>
                </div>
              )}
            </div>
          )}
        </div>
      </Card>

      {/* Review Dialog */}
      <MappingReviewDialog
        isOpen={showReviewDialog}
        onClose={() => setShowReviewDialog(false)}
        suggestion={suggestion}
        onApprove={handleApprove}
        onReject={handleReject}
        approving={approving}
        rejecting={rejecting}
      />
    </>
  );
}
