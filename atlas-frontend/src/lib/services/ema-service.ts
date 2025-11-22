import { apiClient } from '../api-client';
import type {
  EmaMedicine,
  EmaSearchParams,
  EmaStats,
  EmaSyncLog,
  EmaSyncRequest,
  EmaConfigInfo,
  EmaHealthStatus,
  EmaRefreshStatus,
  EmaCleanupResult
} from '@/types/ema';

export class EmaService {
  /**
   * Search EMA pharmaceutical catalog with full-text search and filters
   * @param params - Search parameters including query, filters, and pagination
   * @returns Promise resolving to array of EMA medicines
   */
  static async search(params: EmaSearchParams): Promise<EmaMedicine[]> {
    const searchParams = new URLSearchParams();

    if (params.query) searchParams.append('query', params.query);
    if (params.language) searchParams.append('language', params.language);
    if (params.authorization_status) searchParams.append('authorization_status', params.authorization_status);
    if (params.therapeutic_area) searchParams.append('therapeutic_area', params.therapeutic_area);
    if (params.atc_code) searchParams.append('atc_code', params.atc_code);
    if (params.mah_name) searchParams.append('mah_name', params.mah_name);
    if (params.limit) searchParams.append('limit', params.limit.toString());
    if (params.offset) searchParams.append('offset', params.offset.toString());

    const url = `/api/ema/search${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
    const response = await apiClient.get<EmaMedicine[]>(url);
    return response;
  }

  /**
   * Get medicine by EU number
   * @param euNumber - The EU number of the medicine (format: EU/1/XX/XXX/XXX)
   * @returns Promise resolving to medicine data or null if not found
   */
  static async getByEuNumber(euNumber: string): Promise<EmaMedicine | null> {
    const response = await apiClient.get<EmaMedicine | null>(`/api/ema/eu/${encodeURIComponent(euNumber)}`);
    return response;
  }

  /**
   * Get catalog statistics and metadata
   * @returns Promise resolving to comprehensive catalog statistics
   */
  static async getStats(): Promise<EmaStats> {
    const response = await apiClient.get<EmaStats>('/api/ema/stats');
    return response;
  }

  /**
   * Trigger sync from EMA API (admin only)
   * @param params - Sync parameters
   * @returns Promise resolving to sync log information
   */
  static async triggerSync(params?: EmaSyncRequest): Promise<EmaSyncLog> {
    const searchParams = new URLSearchParams();
    if (params?.language) searchParams.append('language', params.language);
    if (params?.limit) searchParams.append('limit', params.limit.toString());
    if (params?.sync_type) searchParams.append('sync_type', params.sync_type);

    const url = `/api/ema/sync${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
    const response = await apiClient.post<EmaSyncLog>(url, {});
    return response;
  }

  /**
   * Get synchronization logs with pagination
   * @param limit - Number of logs to return (default: 20, max: 100)
   * @param offset - Offset for pagination (default: 0)
   * @returns Promise resolving to array of sync log entries
   */
  static async getSyncLogs(limit: number = 20, offset: number = 0): Promise<EmaSyncLog[]> {
    const searchParams = new URLSearchParams();
    searchParams.append('limit', Math.min(limit, 100).toString());
    searchParams.append('offset', offset.toString());

    const response = await apiClient.get<EmaSyncLog[]>(`/api/ema/sync/logs?${searchParams.toString()}`);
    return response;
  }

  /**
   * Check if catalog needs refresh
   * @param daysThreshold - Number of days to consider data stale (default: 7)
   * @returns Promise resolving to refresh status information
   */
  static async checkRefreshStatus(daysThreshold: number = 7): Promise<EmaRefreshStatus> {
    const searchParams = new URLSearchParams();
    searchParams.append('days_threshold', daysThreshold.toString());

    const response = await apiClient.get<EmaRefreshStatus>(`/api/ema/refresh-status?${searchParams.toString()}`);
    return response;
  }

  /**
   * Get service configuration and supported languages
   * @returns Promise resolving to configuration information
   */
  static async getConfigInfo(): Promise<EmaConfigInfo> {
    const response = await apiClient.get<EmaConfigInfo>('/api/ema/config');
    return response;
  }

  /**
   * Health check endpoint for EMA service
   * @returns Promise resolving to health status
   */
  static async healthCheck(): Promise<EmaHealthStatus> {
    const response = await apiClient.get<EmaHealthStatus>('/api/ema/health');
    return response;
  }

  /**
   * Clean up old sync logs (admin only)
   * @returns Promise resolving to cleanup result
   */
  static async cleanupSyncLogs(): Promise<EmaCleanupResult> {
    const response = await apiClient.post<EmaCleanupResult>('/api/ema/cleanup', {});
    return response;
  }

  // ============================================================================
  // Helper Methods
  // ============================================================================

  /**
   * Validate EU number format
   * @param euNumber - EU number to validate
   * @returns True if valid, false otherwise
   */
  static isValidEuNumber(euNumber: string): boolean {
    if (!euNumber || euNumber.trim() === '') {
      return false;
    }

    // Auto-generated numbers are valid
    if (euNumber.startsWith('AUTO-')) {
      return true;
    }

    // Check standard EU number format
    if (euNumber.startsWith('EU/')) {
      const parts = euNumber.split('/');
      return parts.length >= 4;
    }

    return false;
  }

  /**
   * Get supported language codes
   * @returns Array of supported language codes
   */
  static getSupportedLanguages(): string[] {
    return ['en', 'de', 'fr', 'es', 'it', 'pt', 'nl', 'sv', 'fi', 'da', 'no', 'el'];
  }

  /**
   * Format EU number for display
   * @param euNumber - EU number to format
   * @returns Formatted EU number
   */
  static formatEuNumber(euNumber: string): string {
    if (euNumber.startsWith('AUTO-')) {
      return euNumber;
    }
    return euNumber.toUpperCase();
  }

  /**
   * Get status color for authorization status
   * @param status - Authorization status
   * @returns CSS color class
   */
  static getStatusColor(status?: string): string {
    switch (status?.toLowerCase()) {
      case 'authorized':
      case 'active':
        return 'text-green-600 bg-green-100';
      case 'suspended':
      case 'inactive':
        return 'text-yellow-600 bg-yellow-100';
      case 'withdrawn':
      case 'refused':
        return 'text-red-600 bg-red-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  }

  /**
   * Format sync status for display
   * @param status - Sync status
   * @returns Formatted status string
   */
  static formatSyncStatus(status: string): string {
    switch (status) {
      case 'in_progress':
        return 'In Progress';
      case 'completed':
        return 'Completed';
      case 'failed':
        return 'Failed';
      case 'cancelled':
        return 'Cancelled';
      default:
        return status.charAt(0).toUpperCase() + status.slice(1);
    }
  }

  /**
   * Get color class for sync status
   * @param status - Sync status
   * @returns CSS color class
   */
  static getSyncStatusColor(status: string): string {
    switch (status) {
      case 'completed':
        return 'text-green-600 bg-green-100';
      case 'failed':
        return 'text-red-600 bg-red-100';
      case 'in_progress':
        return 'text-blue-600 bg-blue-100';
      case 'cancelled':
        return 'text-gray-600 bg-gray-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  }

  /**
   * Format language code to display name
   * @param languageCode - Language code
   * @returns Language display name
   */
  static formatLanguageName(languageCode: string): string {
    const languages: Record<string, string> = {
      'en': 'English',
      'de': 'German',
      'fr': 'French',
      'es': 'Spanish',
      'it': 'Italian',
      'pt': 'Portuguese',
      'nl': 'Dutch',
      'sv': 'Swedish',
      'fi': 'Finnish',
      'da': 'Danish',
      'no': 'Norwegian',
      'el': 'Greek'
    };
    return languages[languageCode] || languageCode.toUpperCase();
  }

  /**
   * Format date for display
   * @param dateString - ISO date string
   * @returns Formatted date string
   */
  static formatDate(dateString?: string): string {
    if (!dateString) return 'N/A';
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  }

  /**
   * Format sync duration for display
   * @param processingTimeMs - Processing time in milliseconds
   * @returns Formatted duration string
   */
  static formatSyncDuration(processingTimeMs?: number): string {
    if (!processingTimeMs) return 'N/A';

    const seconds = Math.floor(processingTimeMs / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;

    if (minutes > 0) {
      return `${minutes}m ${remainingSeconds}s`;
    } else {
      return `${seconds}s`;
    }
  }

  /**
   * Generate search suggestions based on common EMA medicine patterns
   * @param query - Current search query
   * @returns Array of search suggestions
   */
  static generateSearchSuggestions(query: string): string[] {
    if (!query || query.length < 2) return [];

    const commonTerms = [
      'paracetamol', 'ibuprofen', 'aspirin', 'amoxicillin', 'metformin',
      'lisinopril', 'atorvastatin', 'simvastatin', 'omeprazole', 'sertraline',
      'tablet', 'solution', 'injection', 'capsule', 'suspension'
    ];

    const lowerQuery = query.toLowerCase();
    return commonTerms
      .filter(term => term.includes(lowerQuery))
      .slice(0, 5)
      .map(term => term.charAt(0).toUpperCase() + term.slice(1));
  }
}

// Export types for convenience
export type {
  EmaMedicine,
  EmaSearchParams,
  EmaStats,
  EmaSyncLog,
  EmaSyncRequest,
  EmaConfigInfo,
  EmaHealthStatus,
  EmaRefreshStatus,
  EmaCleanupResult
};