'use client';

import { X, Check, XCircle, ArrowRight, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import type { MappingSuggestion } from '@/types/erp';
import { getConfidenceColor, getConfidenceLabel } from '@/types/erp';

interface MappingReviewDialogProps {
  isOpen: boolean;
  onClose: () => void;
  suggestion: MappingSuggestion;
  onApprove: () => Promise<void>;
  onReject: () => Promise<void>;
  approving: boolean;
  rejecting: boolean;
}

export function MappingReviewDialog({
  isOpen,
  onClose,
  suggestion,
  onApprove,
  onReject,
  approving,
  rejecting,
}: MappingReviewDialogProps) {
  if (!isOpen) return null;

  const confidenceColor = getConfidenceColor(suggestion.confidence_score);
  const confidenceLabel = getConfidenceLabel(suggestion.confidence_score);

  const confidenceBadgeClass = {
    green: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    red: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
  }[confidenceColor];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-2xl max-w-5xl w-full max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="sticky top-0 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 p-6 flex items-center justify-between z-10">
          <div>
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Mapping Review
            </h2>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Compare products side-by-side before approving
            </p>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="gap-2"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* Confidence Score Banner */}
        <div className={`p-4 ${
          confidenceColor === 'green' ? 'bg-green-50 dark:bg-green-900/20 border-b border-green-200 dark:border-green-800' :
          confidenceColor === 'yellow' ? 'bg-yellow-50 dark:bg-yellow-900/20 border-b border-yellow-200 dark:border-yellow-800' :
          'bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800'
        }`}>
          <div className="flex items-center justify-between max-w-5xl mx-auto">
            <div>
              <Badge className={confidenceBadgeClass}>
                {confidenceLabel}
              </Badge>
              <p className="text-sm mt-2 text-gray-700 dark:text-gray-300">
                <span className="font-semibold">AI Confidence:</span> {(suggestion.confidence_score * 100).toFixed(1)}%
              </p>
            </div>
            <div className="text-right">
              <p className="text-4xl font-bold text-gray-900 dark:text-white">
                {(suggestion.confidence_score * 100).toFixed(0)}%
              </p>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="p-6">
          {/* AI Reasoning */}
          <div className="mb-8 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg border border-purple-200 dark:border-purple-800">
            <h3 className="font-semibold text-purple-900 dark:text-purple-200 mb-2">
              AI Reasoning:
            </h3>
            <p className="text-gray-700 dark:text-gray-300 italic">
              "{suggestion.ai_reasoning}"
            </p>
          </div>

          {/* Side-by-Side Comparison */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
            {/* Atlas Product */}
            <div className="p-6 bg-blue-50 dark:bg-blue-900/20 rounded-lg border-2 border-blue-200 dark:border-blue-800">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
                <span className="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center text-sm font-bold">
                  A
                </span>
                Atlas Product
              </h3>

              <div className="space-y-3">
                <div>
                  <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Product Name
                  </label>
                  <p className="text-gray-900 dark:text-white font-semibold">
                    {suggestion.atlas_product.product_name}
                  </p>
                </div>

                {suggestion.atlas_product.ndc && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      NDC
                    </label>
                    <p className="text-gray-900 dark:text-white font-mono">
                      {suggestion.atlas_product.ndc}
                    </p>
                  </div>
                )}

                {suggestion.atlas_product.manufacturer && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Manufacturer
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.atlas_product.manufacturer}
                    </p>
                  </div>
                )}

                {suggestion.atlas_product.strength && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Strength
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.atlas_product.strength}
                    </p>
                  </div>
                )}

                {suggestion.atlas_product.dosage_form && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Dosage Form
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.atlas_product.dosage_form}
                    </p>
                  </div>
                )}

                {suggestion.atlas_product.package_size && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Package Size
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.atlas_product.package_size}
                    </p>
                  </div>
                )}

                <div>
                  <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Atlas SKU
                  </label>
                  <p className="text-gray-900 dark:text-white font-mono text-sm">
                    {suggestion.atlas_product.atlas_sku}
                  </p>
                </div>
              </div>
            </div>

            {/* ERP Product */}
            <div className="p-6 bg-indigo-50 dark:bg-indigo-900/20 rounded-lg border-2 border-indigo-200 dark:border-indigo-800">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
                <span className="w-8 h-8 rounded-full bg-indigo-600 text-white flex items-center justify-center text-sm font-bold">
                  E
                </span>
                {suggestion.erp_product.system} Product
              </h3>

              <div className="space-y-3">
                <div>
                  <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Item Name
                  </label>
                  <p className="text-gray-900 dark:text-white font-semibold">
                    {suggestion.erp_product.item_name || 'N/A'}
                  </p>
                </div>

                <div>
                  <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Item ID
                  </label>
                  <p className="text-gray-900 dark:text-white font-mono">
                    {suggestion.erp_product.item_id}
                  </p>
                </div>

                {suggestion.erp_product.description && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Description
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.erp_product.description}
                    </p>
                  </div>
                )}

                {suggestion.erp_product.manufacturer && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Manufacturer
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.erp_product.manufacturer}
                    </p>
                  </div>
                )}

                {suggestion.erp_product.category && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Category
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.erp_product.category}
                    </p>
                  </div>
                )}

                {suggestion.erp_product.unit_of_measure && (
                  <div>
                    <label className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                      Unit of Measure
                    </label>
                    <p className="text-gray-900 dark:text-white">
                      {suggestion.erp_product.unit_of_measure}
                    </p>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Matching Factors */}
          <div className="mb-8">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Matching Factors
            </h3>

            <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
              {suggestion.matching_factors.ndc_match !== undefined && (
                <div className={`p-4 rounded-lg border-2 ${
                  suggestion.matching_factors.ndc_match
                    ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                    : 'bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-700'
                }`}>
                  <div className="flex items-center gap-2 mb-1">
                    {suggestion.matching_factors.ndc_match ? (
                      <Check className="h-5 w-5 text-green-600" />
                    ) : (
                      <XCircle className="h-5 w-5 text-gray-400" />
                    )}
                    <span className="font-semibold text-gray-900 dark:text-white">
                      NDC Match
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {suggestion.matching_factors.ndc_match ? 'Codes match exactly' : 'Different codes'}
                  </p>
                </div>
              )}

              {suggestion.matching_factors.name_similarity !== undefined && (
                <div className="p-4 rounded-lg border-2 bg-blue-50 dark:bg-blue-900/20 border-blue-300 dark:border-blue-700">
                  <div className="flex items-center gap-2 mb-1">
                    <div className="w-5 h-5 rounded-full bg-blue-600" />
                    <span className="font-semibold text-gray-900 dark:text-white">
                      Name Similarity
                    </span>
                  </div>
                  <p className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                    {(suggestion.matching_factors.name_similarity * 100).toFixed(0)}%
                  </p>
                </div>
              )}

              {suggestion.matching_factors.manufacturer_match !== undefined && (
                <div className={`p-4 rounded-lg border-2 ${
                  suggestion.matching_factors.manufacturer_match
                    ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                    : 'bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-700'
                }`}>
                  <div className="flex items-center gap-2 mb-1">
                    {suggestion.matching_factors.manufacturer_match ? (
                      <Check className="h-5 w-5 text-green-600" />
                    ) : (
                      <XCircle className="h-5 w-5 text-gray-400" />
                    )}
                    <span className="font-semibold text-gray-900 dark:text-white">
                      Manufacturer
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {suggestion.matching_factors.manufacturer_match ? 'Same manufacturer' : 'Different manufacturer'}
                  </p>
                </div>
              )}

              {suggestion.matching_factors.strength_match !== undefined && (
                <div className={`p-4 rounded-lg border-2 ${
                  suggestion.matching_factors.strength_match
                    ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                    : 'bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-700'
                }`}>
                  <div className="flex items-center gap-2 mb-1">
                    {suggestion.matching_factors.strength_match ? (
                      <Check className="h-5 w-5 text-green-600" />
                    ) : (
                      <XCircle className="h-5 w-5 text-gray-400" />
                    )}
                    <span className="font-semibold text-gray-900 dark:text-white">
                      Strength Match
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {suggestion.matching_factors.strength_match ? 'Same strength' : 'Different strength'}
                  </p>
                </div>
              )}

              {suggestion.matching_factors.dosage_form_match !== undefined && (
                <div className={`p-4 rounded-lg border-2 ${
                  suggestion.matching_factors.dosage_form_match
                    ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                    : 'bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-700'
                }`}>
                  <div className="flex items-center gap-2 mb-1">
                    {suggestion.matching_factors.dosage_form_match ? (
                      <Check className="h-5 w-5 text-green-600" />
                    ) : (
                      <XCircle className="h-5 w-5 text-gray-400" />
                    )}
                    <span className="font-semibold text-gray-900 dark:text-white">
                      Dosage Form
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {suggestion.matching_factors.dosage_form_match ? 'Same form' : 'Different form'}
                  </p>
                </div>
              )}
            </div>

            {suggestion.matching_factors.additional_notes && (
              <div className="mt-4 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                <h4 className="font-medium text-gray-900 dark:text-white mb-2">
                  Additional Notes:
                </h4>
                <p className="text-gray-600 dark:text-gray-400">
                  {suggestion.matching_factors.additional_notes}
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Footer Actions */}
        <div className="sticky bottom-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 p-6 flex items-center justify-end gap-3">
          <Button
            variant="outline"
            onClick={onClose}
            disabled={approving || rejecting}
          >
            Cancel
          </Button>

          <Button
            variant="destructive"
            onClick={async () => {
              await onReject();
              onClose();
            }}
            disabled={approving || rejecting}
            className="gap-2"
          >
            {rejecting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <XCircle className="h-4 w-4" />
            )}
            Reject Mapping
          </Button>

          <Button
            onClick={async () => {
              await onApprove();
              onClose();
            }}
            disabled={approving || rejecting}
            className="gap-2 bg-green-600 hover:bg-green-700"
          >
            {approving ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Check className="h-4 w-4" />
            )}
            Approve Mapping
          </Button>
        </div>
      </div>
    </div>
  );
}
