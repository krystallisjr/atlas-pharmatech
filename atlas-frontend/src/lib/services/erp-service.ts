// ERP Integration Service Layer
// Production-ready service for NetSuite and SAP API integration

import { apiClient } from '../api-client';
import type {
  ErpConnection,
  CreateConnectionRequest,
  ConnectionTestResult,
  SyncLog,
  TriggerSyncRequest,
  InventoryMapping,
  MappingSuggestion,
  ReviewMappingSuggestionRequest,
  MappingStatus,
  MappingDiscoveryResponse,
  SyncInsight,
  ConflictResolutionResponse,
  ResolveConflictsRequest,
} from '@/types/erp';

export class ErpService {
  // ============================================================================
  // Connection Management
  // ============================================================================

  /**
   * Create a new ERP connection (NetSuite or SAP)
   */
  static async createConnection(
    data: CreateConnectionRequest
  ): Promise<ErpConnection> {
    const response = await apiClient.post<ErpConnection>(
      '/api/erp/connections',
      data
    );
    return response.data;
  }

  /**
   * Get all ERP connections for current user
   */
  static async listConnections(): Promise<ErpConnection[]> {
    const response = await apiClient.get<ErpConnection[]>(
      '/api/erp/connections'
    );
    return response.data;
  }

  /**
   * Get a single ERP connection by ID
   */
  static async getConnection(connectionId: string): Promise<ErpConnection> {
    const response = await apiClient.get<ErpConnection>(
      `/api/erp/connections/${connectionId}`
    );
    return response.data;
  }

  /**
   * Delete an ERP connection
   */
  static async deleteConnection(connectionId: string): Promise<void> {
    await apiClient.delete(`/api/erp/connections/${connectionId}`);
  }

  /**
   * Test ERP connection credentials
   * Validates OAuth credentials and API accessibility
   */
  static async testConnection(
    connectionId: string
  ): Promise<ConnectionTestResult> {
    const response = await apiClient.post<ConnectionTestResult>(
      `/api/erp/connections/${connectionId}/test`,
      {}
    );
    return response.data;
  }

  // ============================================================================
  // Sync Operations
  // ============================================================================

  /**
   * Trigger a manual sync operation
   */
  static async triggerSync(
    connectionId: string,
    request?: TriggerSyncRequest
  ): Promise<SyncLog> {
    const response = await apiClient.post<SyncLog>(
      `/api/erp/connections/${connectionId}/sync`,
      request || {}
    );
    return response.data;
  }

  /**
   * Get sync logs for a connection
   */
  static async getSyncLogs(connectionId: string): Promise<SyncLog[]> {
    const response = await apiClient.get<SyncLog[]>(
      `/api/erp/connections/${connectionId}/sync-logs`
    );
    return response.data;
  }

  /**
   * Get AI analysis of a failed sync
   * Returns plain-English error explanation and recommendations
   */
  static async getSync Analysis(syncLogId: string): Promise<SyncInsight> {
    const response = await apiClient.get<SyncInsight>(
      `/api/erp/sync-logs/${syncLogId}/ai-analysis`
    );
    return response.data;
  }

  // ============================================================================
  // Mapping Management
  // ============================================================================

  /**
   * Get all inventory mappings for a connection
   */
  static async getMappings(connectionId: string): Promise<InventoryMapping[]> {
    const response = await apiClient.get<InventoryMapping[]>(
      `/api/erp/connections/${connectionId}/mappings`
    );
    return response.data;
  }

  /**
   * Delete an inventory mapping
   */
  static async deleteMapping(mappingId: string): Promise<void> {
    await apiClient.delete(`/api/erp/mappings/${mappingId}`);
  }

  /**
   * Get mapping status (progress percentage, counts)
   */
  static async getMappingStatus(connectionId: string): Promise<MappingStatus> {
    const response = await apiClient.get<MappingStatus>(
      `/api/erp/connections/${connectionId}/mapping-status`
    );
    return response.data;
  }

  // ============================================================================
  // AI-Powered Features
  // ============================================================================

  /**
   * Trigger AI auto-discovery of inventory mappings
   * Uses Claude AI to match Atlas inventory with ERP inventory
   * based on NDC codes, product names, manufacturers, etc.
   */
  static async autoDiscoverMappings(
    connectionId: string
  ): Promise<MappingDiscoveryResponse> {
    const response = await apiClient.post<MappingDiscoveryResponse>(
      `/api/erp/connections/${connectionId}/auto-discover-mappings`,
      {}
    );
    return response.data;
  }

  /**
   * Get AI-generated mapping suggestions
   */
  static async getMappingSuggestions(
    connectionId: string
  ): Promise<MappingSuggestion[]> {
    const response = await apiClient.get<MappingSuggestion[]>(
      `/api/erp/connections/${connectionId}/mapping-suggestions`
    );
    return response.data;
  }

  /**
   * Review (approve or reject) a mapping suggestion
   */
  static async reviewMappingSuggestion(
    connectionId: string,
    suggestionId: string,
    request: ReviewMappingSuggestionRequest
  ): Promise<InventoryMapping> {
    const response = await apiClient.post<InventoryMapping>(
      `/api/erp/connections/${connectionId}/mapping-suggestions/${suggestionId}/review`,
      request
    );
    return response.data;
  }

  /**
   * Resolve data conflicts using AI recommendations
   * AI analyzes timestamps, transaction history, and business rules
   * to recommend which system's data is correct
   */
  static async resolveConflicts(
    connectionId: string,
    request: ResolveConflictsRequest
  ): Promise<ConflictResolutionResponse> {
    const response = await apiClient.post<ConflictResolutionResponse>(
      `/api/erp/connections/${connectionId}/resolve-conflicts`,
      request
    );
    return response.data;
  }

  // ============================================================================
  // Webhook Management
  // ============================================================================

  /**
   * Get webhook URL for NetSuite
   */
  static getNetSuiteWebhookUrl(connectionId: string): string {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || 'https://localhost:8443';
    return `${baseUrl}/api/erp/webhooks/netsuite/${connectionId}`;
  }

  /**
   * Get webhook URL for SAP
   */
  static getSapWebhookUrl(connectionId: string): string {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || 'https://localhost:8443';
    return `${baseUrl}/api/erp/webhooks/sap/${connectionId}`;
  }
}
