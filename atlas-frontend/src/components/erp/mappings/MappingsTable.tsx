'use client';

import { useState } from 'react';
import { Trash2, Search, ArrowRight, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import type { InventoryMapping } from '@/types/erp';
import { getConfidenceColor, getConfidenceLabel } from '@/types/erp';

interface MappingsTableProps {
  mappings: InventoryMapping[];
  onDelete: (mappingId: string) => Promise<void>;
}

export function MappingsTable({ mappings, onDelete }: MappingsTableProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const handleDelete = async (mappingId: string) => {
    setDeletingId(mappingId);
    try {
      await onDelete(mappingId);
    } finally {
      setDeletingId(null);
    }
  };

  // Filter mappings based on search query
  const filteredMappings = mappings.filter(mapping => {
    const query = searchQuery.toLowerCase();
    return (
      mapping.atlas_product.product_name.toLowerCase().includes(query) ||
      mapping.atlas_product.atlas_sku.toLowerCase().includes(query) ||
      (mapping.atlas_product.ndc && mapping.atlas_product.ndc.toLowerCase().includes(query)) ||
      mapping.erp_product.item_id.toLowerCase().includes(query) ||
      (mapping.erp_product.item_name && mapping.erp_product.item_name.toLowerCase().includes(query))
    );
  });

  return (
    <div>
      {/* Search */}
      <div className="mb-4">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
          <Input
            type="text"
            placeholder="Search mappings by product name, SKU, NDC, or Item ID..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      {/* Table */}
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Atlas Product
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-16">

                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  ERP Product
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-32">
                  Confidence
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-40">
                  Date Approved
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-24">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
              {filteredMappings.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-6 py-8 text-center text-gray-500 dark:text-gray-400">
                    {searchQuery ? 'No mappings match your search' : 'No mappings yet'}
                  </td>
                </tr>
              ) : (
                filteredMappings.map((mapping) => {
                  const confidenceColor = getConfidenceColor(mapping.confidence_score);
                  const confidenceLabel = getConfidenceLabel(mapping.confidence_score);

                  const confidenceBadgeClass = {
                    green: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
                    yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
                    red: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
                  }[confidenceColor];

                  return (
                    <tr key={mapping.id} className="hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors">
                      {/* Atlas Product */}
                      <td className="px-6 py-4">
                        <div>
                          <p className="font-medium text-gray-900 dark:text-white">
                            {mapping.atlas_product.product_name}
                          </p>
                          <div className="text-sm text-gray-500 dark:text-gray-400 space-y-0.5 mt-1">
                            <p>SKU: {mapping.atlas_product.atlas_sku}</p>
                            {mapping.atlas_product.ndc && (
                              <p>NDC: {mapping.atlas_product.ndc}</p>
                            )}
                          </div>
                        </div>
                      </td>

                      {/* Arrow */}
                      <td className="px-6 py-4 text-center">
                        <ArrowRight className="h-5 w-5 text-gray-400 mx-auto" />
                      </td>

                      {/* ERP Product */}
                      <td className="px-6 py-4">
                        <div>
                          <p className="font-medium text-gray-900 dark:text-white">
                            {mapping.erp_product.item_name || mapping.erp_product.item_id}
                          </p>
                          <div className="text-sm text-gray-500 dark:text-gray-400 space-y-0.5 mt-1">
                            <p>Item ID: {mapping.erp_product.item_id}</p>
                            {mapping.erp_product.manufacturer && (
                              <p>Mfr: {mapping.erp_product.manufacturer}</p>
                            )}
                          </div>
                        </div>
                      </td>

                      {/* Confidence */}
                      <td className="px-6 py-4 text-center">
                        <Badge className={confidenceBadgeClass}>
                          {(mapping.confidence_score * 100).toFixed(0)}%
                        </Badge>
                      </td>

                      {/* Date Approved */}
                      <td className="px-6 py-4 text-center text-sm text-gray-500 dark:text-gray-400">
                        {new Date(mapping.approved_at).toLocaleDateString('en-US', {
                          year: 'numeric',
                          month: 'short',
                          day: 'numeric',
                        })}
                      </td>

                      {/* Actions */}
                      <td className="px-6 py-4 text-center">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleDelete(mapping.id)}
                          disabled={deletingId === mapping.id}
                          className="gap-2 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20"
                        >
                          {deletingId === mapping.id ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <Trash2 className="h-4 w-4" />
                          )}
                        </Button>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Results Count */}
      {searchQuery && (
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-3">
          Showing {filteredMappings.length} of {mappings.length} mappings
        </p>
      )}
    </div>
  );
}
