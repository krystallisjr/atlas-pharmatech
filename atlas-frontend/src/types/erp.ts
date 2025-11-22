// ERP Integration Type Definitions
// Production-ready types for NetSuite and SAP integration

export type ErpType = 'netsuite' | 'sap_s4hana';

export type ConnectionStatus = 'active' | 'inactive' | 'error' | 'testing';

export type SyncDirection = 'atlas_to_erp' | 'erp_to_atlas' | 'bidirectional';

export type SyncStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'partial';

export type MappingStatusType = 'suggested' | 'approved' | 'rejected' | 'active';

export type ConflictResolutionStrategy = 'atlas_wins' | 'erp_wins' | 'manual_review' | 'merge' | 'reject_sync';

export type RiskLevel = 'low' | 'medium' | 'high' | 'critical';

// ============================================================================
// Connection Types
// ============================================================================

export interface NetSuiteConfig {
  account_id: string;
  consumer_key: string;
  consumer_secret: string;
  token_id: string;
  token_secret: string;
  realm?: string;
}

export interface SapConfig {
  base_url: string;
  client_id: string;
  client_secret: string;
  token_endpoint: string;
  environment: 'cloud' | 'on_premise';
  plant?: string;
  company_code?: string;
}

export interface ErpConnection {
  id: string;
  user_id: string;
  connection_name: string;
  erp_type: ErpType;
  status: ConnectionStatus;

  // Sync configuration
  sync_enabled: boolean;
  sync_frequency_minutes: number;
  last_sync_at?: string;
  last_sync_status?: string;

  // Feature flags
  sync_stock_levels: boolean;
  sync_product_master: boolean;
  sync_transactions: boolean;
  sync_lot_batch: boolean;

  // Sync preferences
  default_sync_direction: SyncDirection;
  conflict_resolution: ConflictResolutionStrategy;

  // Metadata
  created_at: string;
  updated_at: string;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
  details?: {
    api_reachable: boolean;
    authentication_valid: boolean;
    permissions_verified: boolean;
  };
}

// ============================================================================
// Sync Types
// ============================================================================

export interface SyncLog {
  id: string;
  erp_connection_id: string;
  sync_direction: SyncDirection;
  status: SyncStatus;

  // Statistics
  items_synced: number;
  items_failed: number;
  duration_seconds?: number;

  // Error details
  error_message?: string;
  error_details?: Record<string, any>;

  // Timestamps
  started_at: string;
  completed_at?: string;
}

export interface SyncInsight {
  insight_type: 'error_explanation' | 'performance_analysis' | 'data_quality' | 'anomaly_detection' | 'success_summary';
  severity: 'info' | 'warning' | 'error' | 'critical';
  title: string;
  explanation: string;
  recommendations: Recommendation[];
  actionable: boolean;
}

export interface Recommendation {
  action: string;
  priority: 'high' | 'medium' | 'low';
  description: string;
}

// ============================================================================
// Mapping Types
// ============================================================================

export interface InventoryMapping {
  id: string;
  erp_connection_id: string;
  atlas_inventory_id: string;
  erp_item_id: string;
  erp_item_name: string;
  erp_item_description?: string;
  mapping_status: MappingStatus;
  confidence_score?: number;
  created_at: string;
  updated_at: string;
}

export interface MappingSuggestion {
  id: string;
  erp_connection_id: string;
  atlas_inventory_id: string;
  erp_item_id: string;
  erp_item_name: string;
  erp_item_description?: string;
  confidence_score: number;
  ai_reasoning: string;
  matching_factors: {
    ndc_match?: boolean;
    name_similarity?: number;
    manufacturer_match?: boolean;
    strength_match?: boolean;
  };
  status: MappingStatus;
  created_at: string;
}

export interface MappingDiscoveryResponse {
  mappings: MappingSuggestion[];
  unmapped_atlas_items: string[];
  unmapped_erp_items: string[];
  warnings: string[];
}

export interface MappingStatus {
  total_atlas_items: number;
  total_erp_items: number;
  mapped_count: number;
  suggested_count: number;
  unmapped_atlas_count: number;
  unmapped_erp_count: number;
  mapping_percentage: number;
}

// ============================================================================
// Conflict Resolution Types
// ============================================================================

export interface ConflictData {
  atlas_inventory_id: string;
  erp_item_id: string;
  conflict_type: 'quantity_mismatch' | 'price_mismatch' | 'data_quality' | 'timestamp_conflict';
  atlas_value: any;
  erp_value: any;
  atlas_updated_at?: string;
  erp_updated_at?: string;
}

export interface ConflictResolutionSuggestion {
  conflict_type: string;
  suggested_resolution: ConflictResolutionStrategy;
  confidence_score: number;
  risk_level: RiskLevel;
  reasoning: string;
  evidence: {
    atlas_timestamp?: string;
    erp_timestamp?: string;
    recent_atlas_transactions?: string;
    recent_erp_transactions?: string;
  };
}

export interface ConflictResolutionResponse {
  resolutions: ConflictResolutionSuggestion[];
}

// ============================================================================
// Request/Response Types
// ============================================================================

export interface CreateConnectionRequest {
  connection_name: string;
  erp_type: ErpType;

  // NetSuite fields (required if erp_type === 'netsuite')
  netsuite_account_id?: string;
  netsuite_consumer_key?: string;
  netsuite_consumer_secret?: string;
  netsuite_token_id?: string;
  netsuite_token_secret?: string;
  netsuite_realm?: string;

  // SAP fields (required if erp_type === 'sap_s4hana')
  sap_base_url?: string;
  sap_client_id?: string;
  sap_client_secret?: string;
  sap_token_endpoint?: string;
  sap_environment?: 'cloud' | 'on_premise';
  sap_plant?: string;
  sap_company_code?: string;

  // Sync configuration (optional, has defaults)
  sync_enabled?: boolean;
  sync_frequency_minutes?: number;
  default_sync_direction?: SyncDirection;
  conflict_resolution?: ConflictResolutionStrategy;
}

export interface TriggerSyncRequest {
  sync_direction?: SyncDirection;
}

export interface ReviewMappingSuggestionRequest {
  approved: boolean;
}

export interface ResolveConflictsRequest {
  conflicts: ConflictData[];
}

// ============================================================================
// UI Helper Types
// ============================================================================

export interface ErpSystemInfo {
  type: ErpType;
  name: string;
  description: string;
  logoUrl?: string;
  color: string;
  features: string[];
}

export const ERP_SYSTEMS: Record<ErpType, ErpSystemInfo> = {
  netsuite: {
    type: 'netsuite',
    name: 'NetSuite',
    description: 'Oracle NetSuite Cloud ERP for enterprise resource planning',
    color: 'blue',
    features: [
      'Real-time inventory sync',
      'OAuth 1.0 secure authentication',
      'Automated purchase orders',
      'Custom pharmaceutical fields'
    ]
  },
  sap_s4hana: {
    type: 'sap_s4hana',
    name: 'SAP S/4HANA',
    description: 'SAP next-generation ERP with OData API integration',
    color: 'indigo',
    features: [
      'Material master data sync',
      'Stock level monitoring',
      'OAuth 2.0 authentication',
      'Multi-plant support'
    ]
  }
};

export function getConfidenceColor(score: number): string {
  if (score >= 0.9) return 'green';
  if (score >= 0.7) return 'yellow';
  return 'red';
}

export function getConfidenceLabel(score: number): string {
  if (score >= 0.9) return 'High Confidence';
  if (score >= 0.7) return 'Medium Confidence';
  return 'Low Confidence';
}

export function getSyncStatusColor(status: SyncStatus): string {
  switch (status) {
    case 'completed': return 'green';
    case 'failed': return 'red';
    case 'in_progress': return 'blue';
    case 'partial': return 'yellow';
    default: return 'gray';
  }
}
