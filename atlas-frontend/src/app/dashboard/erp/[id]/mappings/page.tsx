'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, Sparkles, Filter, CheckCircle, XCircle, Loader2 } from 'lucide-react';
import { ErpService } from '@/lib/services';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { ErpConnection, MappingSuggestion, InventoryMapping, MappingStatus } from '@/types/erp';
import { toast } from 'react-toastify';
import { MappingSuggestionCard } from '@/components/erp/mappings/MappingSuggestionCard';
import { AutoDiscoveryButton } from '@/components/erp/mappings/AutoDiscoveryButton';
import { MappingStatusIndicator } from '@/components/erp/mappings/MappingStatusIndicator';
import { MappingsTable } from '@/components/erp/mappings/MappingsTable';

type ConfidenceFilter = 'all' | 'high' | 'medium' | 'low';

export default function MappingsPage({ params }: { params: { id: string } }) {
  const router = useRouter();
  const [connection, setConnection] = useState<ErpConnection | null>(null);
  const [mappingStatus, setMappingStatus] = useState<MappingStatus | null>(null);
  const [suggestions, setSuggestions] = useState<MappingSuggestion[]>([]);
  const [approvedMappings, setApprovedMappings] = useState<InventoryMapping[]>([]);
  const [loading, setLoading] = useState(true);
  const [suggestionsLoading, setSuggestionsLoading] = useState(false);
  const [confidenceFilter, setConfidenceFilter] = useState<ConfidenceFilter>('all');
  const [bulkSelecting, setBulkSelecting] = useState(false);
  const [selectedSuggestions, setSelectedSuggestions] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadData();
  }, [params.id]);

  const loadData = async () => {
    try {
      setLoading(true);
      const [connData, statusData, suggestionsData, mappingsData] = await Promise.all([
        ErpService.getConnection(params.id),
        ErpService.getMappingStatus(params.id),
        ErpService.getMappingSuggestions(params.id).catch(() => []),
        ErpService.getMappings(params.id).catch(() => []),
      ]);

      setConnection(connData);
      setMappingStatus(statusData);
      setSuggestions(suggestionsData);
      setApprovedMappings(mappingsData);
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to load mappings');
      router.push('/dashboard/erp');
    } finally {
      setLoading(false);
    }
  };

  const handleDiscoveryComplete = async () => {
    setSuggestionsLoading(true);
    try {
      const [suggestionsData, statusData] = await Promise.all([
        ErpService.getMappingSuggestions(params.id),
        ErpService.getMappingStatus(params.id),
      ]);
      setSuggestions(suggestionsData);
      setMappingStatus(statusData);
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to load suggestions');
    } finally {
      setSuggestionsLoading(false);
    }
  };

  const handleApprove = async (suggestionId: string) => {
    try {
      const mapping = await ErpService.reviewMappingSuggestion(suggestionId, 'approve');

      // Remove from suggestions and add to approved
      setSuggestions(prev => prev.filter(s => s.id !== suggestionId));
      setApprovedMappings(prev => [...prev, mapping]);

      // Update mapping status
      const statusData = await ErpService.getMappingStatus(params.id);
      setMappingStatus(statusData);

      toast.success('Mapping approved successfully');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to approve mapping');
    }
  };

  const handleReject = async (suggestionId: string) => {
    try {
      await ErpService.reviewMappingSuggestion(suggestionId, 'reject');

      // Remove from suggestions
      setSuggestions(prev => prev.filter(s => s.id !== suggestionId));

      toast.success('Mapping rejected');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to reject mapping');
    }
  };

  const handleBulkApprove = async () => {
    if (selectedSuggestions.size === 0) {
      toast.warning('No suggestions selected');
      return;
    }

    const approvePromises = Array.from(selectedSuggestions).map(id =>
      ErpService.reviewMappingSuggestion(id, 'approve')
    );

    try {
      await Promise.all(approvePromises);

      // Reload data
      await loadData();
      setSelectedSuggestions(new Set());
      setBulkSelecting(false);

      toast.success(`${approvePromises.length} mappings approved`);
    } catch (error: any) {
      toast.error('Some mappings failed to approve');
      await loadData(); // Reload to get accurate state
    }
  };

  const handleBulkReject = async () => {
    if (selectedSuggestions.size === 0) {
      toast.warning('No suggestions selected');
      return;
    }

    const rejectPromises = Array.from(selectedSuggestions).map(id =>
      ErpService.reviewMappingSuggestion(id, 'reject')
    );

    try {
      await Promise.all(rejectPromises);

      // Reload data
      await loadData();
      setSelectedSuggestions(new Set());
      setBulkSelecting(false);

      toast.success(`${rejectPromises.length} mappings rejected`);
    } catch (error: any) {
      toast.error('Some mappings failed to reject');
      await loadData(); // Reload to get accurate state
    }
  };

  const handleMappingDelete = async (mappingId: string) => {
    if (!confirm('Are you sure you want to delete this mapping?')) {
      return;
    }

    try {
      await ErpService.deleteMapping(mappingId);
      setApprovedMappings(prev => prev.filter(m => m.id !== mappingId));

      // Update mapping status
      const statusData = await ErpService.getMappingStatus(params.id);
      setMappingStatus(statusData);

      toast.success('Mapping deleted');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Failed to delete mapping');
    }
  };

  const toggleSuggestionSelection = (suggestionId: string) => {
    setSelectedSuggestions(prev => {
      const next = new Set(prev);
      if (next.has(suggestionId)) {
        next.delete(suggestionId);
      } else {
        next.add(suggestionId);
      }
      return next;
    });
  };

  const selectAllFiltered = () => {
    const filtered = getFilteredSuggestions();
    setSelectedSuggestions(new Set(filtered.map(s => s.id)));
  };

  const deselectAll = () => {
    setSelectedSuggestions(new Set());
  };

  const getFilteredSuggestions = (): MappingSuggestion[] => {
    if (confidenceFilter === 'all') {
      return suggestions;
    }

    return suggestions.filter(s => {
      if (confidenceFilter === 'high') return s.confidence_score >= 0.9;
      if (confidenceFilter === 'medium') return s.confidence_score >= 0.7 && s.confidence_score < 0.9;
      if (confidenceFilter === 'low') return s.confidence_score < 0.7;
      return true;
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto mb-3" />
          <p className="text-gray-600 dark:text-gray-400">Loading mappings...</p>
        </div>
      </div>
    );
  }

  if (!connection || !mappingStatus) {
    return null;
  }

  const filteredSuggestions = getFilteredSuggestions();

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Button
          variant="ghost"
          onClick={() => router.push(`/dashboard/erp/${params.id}`)}
          className="gap-2 mb-4"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Connection
        </Button>

        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              AI Mapping Discovery
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              {connection.connection_name}
            </p>
          </div>

          <AutoDiscoveryButton
            connectionId={params.id}
            onDiscoveryComplete={handleDiscoveryComplete}
          />
        </div>
      </div>

      {/* Mapping Status */}
      <MappingStatusIndicator
        mapped={mappingStatus.mapped_count}
        total={mappingStatus.total_atlas_items}
        percentage={mappingStatus.mapping_percentage}
        suggested={mappingStatus.suggested_count}
      />

      {/* Suggestions Section */}
      {suggestions.length > 0 && (
        <div className="mb-8">
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-4">
              <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
                AI Suggestions
              </h2>
              <Badge className="bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">
                {filteredSuggestions.length} {filteredSuggestions.length === 1 ? 'suggestion' : 'suggestions'}
              </Badge>
            </div>

            <div className="flex items-center gap-3">
              {/* Bulk Selection Toggle */}
              <Button
                variant={bulkSelecting ? 'default' : 'outline'}
                size="sm"
                onClick={() => {
                  setBulkSelecting(!bulkSelecting);
                  if (bulkSelecting) {
                    setSelectedSuggestions(new Set());
                  }
                }}
              >
                {bulkSelecting ? 'Cancel Bulk Select' : 'Bulk Select'}
              </Button>

              {/* Confidence Filter */}
              <div className="flex items-center gap-2">
                <Filter className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                <Select value={confidenceFilter} onValueChange={(value) => setConfidenceFilter(value as ConfidenceFilter)}>
                  <SelectTrigger className="w-36">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Confidence</SelectItem>
                    <SelectItem value="high">High (&gt;90%)</SelectItem>
                    <SelectItem value="medium">Medium (70-90%)</SelectItem>
                    <SelectItem value="low">Low (&lt;70%)</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          {/* Bulk Actions */}
          {bulkSelecting && (
            <Card className="p-4 mb-6 bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <span className="text-sm font-medium text-gray-900 dark:text-white">
                    {selectedSuggestions.size} selected
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={selectAllFiltered}
                  >
                    Select All ({filteredSuggestions.length})
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={deselectAll}
                    disabled={selectedSuggestions.size === 0}
                  >
                    Deselect All
                  </Button>
                </div>

                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    onClick={handleBulkApprove}
                    disabled={selectedSuggestions.size === 0}
                    className="gap-2 bg-green-600 hover:bg-green-700"
                  >
                    <CheckCircle className="h-4 w-4" />
                    Approve Selected
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    onClick={handleBulkReject}
                    disabled={selectedSuggestions.size === 0}
                    className="gap-2"
                  >
                    <XCircle className="h-4 w-4" />
                    Reject Selected
                  </Button>
                </div>
              </div>
            </Card>
          )}

          {/* Suggestions List */}
          {suggestionsLoading ? (
            <div className="flex items-center justify-center py-12">
              <div className="text-center">
                <Loader2 className="h-8 w-8 animate-spin text-purple-600 mx-auto mb-3" />
                <p className="text-gray-600 dark:text-gray-400">Loading suggestions...</p>
              </div>
            </div>
          ) : filteredSuggestions.length > 0 ? (
            <div className="space-y-4">
              {filteredSuggestions.map((suggestion) => (
                <MappingSuggestionCard
                  key={suggestion.id}
                  suggestion={suggestion}
                  onApprove={() => handleApprove(suggestion.id)}
                  onReject={() => handleReject(suggestion.id)}
                  bulkSelecting={bulkSelecting}
                  isSelected={selectedSuggestions.has(suggestion.id)}
                  onToggleSelect={() => toggleSuggestionSelection(suggestion.id)}
                />
              ))}
            </div>
          ) : (
            <Card className="p-12 text-center">
              <Sparkles className="h-12 w-12 text-gray-400 dark:text-gray-600 mx-auto mb-3" />
              <p className="text-gray-600 dark:text-gray-400">
                No suggestions match the selected filter
              </p>
            </Card>
          )}
        </div>
      )}

      {/* Empty State */}
      {suggestions.length === 0 && (
        <Card className="p-12 text-center bg-gradient-to-br from-purple-50 to-indigo-50 dark:from-gray-800 dark:to-gray-900 border-2 border-dashed mb-8">
          <Sparkles className="h-16 w-16 text-purple-600 dark:text-purple-400 mx-auto mb-4" />
          <h3 className="text-xl font-bold text-gray-900 dark:text-white mb-2">
            No AI Suggestions Yet
          </h3>
          <p className="text-gray-600 dark:text-gray-400 mb-6 max-w-md mx-auto">
            Click "Auto-Discover with AI" to let Claude analyze your inventory and find intelligent matches
            between Atlas products and your ERP system.
          </p>
          <AutoDiscoveryButton
            connectionId={params.id}
            onDiscoveryComplete={handleDiscoveryComplete}
          />
        </Card>
      )}

      {/* Approved Mappings Table */}
      {approvedMappings.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Approved Mappings
            </h2>
            <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
              {approvedMappings.length} {approvedMappings.length === 1 ? 'mapping' : 'mappings'}
            </Badge>
          </div>

          <MappingsTable
            mappings={approvedMappings}
            onDelete={handleMappingDelete}
          />
        </div>
      )}
    </div>
  );
}
