export interface EmaMedicine {
  id: string;
  eu_number: string;
  product_name: string;
  inn_name?: string;
  mah_name: string;
  pharmaceutical_form?: string;
  strength?: string;
  authorization_status?: string;
  therapeutic_area?: string;
  atc_code?: string;
  orphan_designation: boolean;
  language_code: string;
}

export interface EmaSearchParams {
  query?: string;
  language?: string;
  authorization_status?: string;
  therapeutic_area?: string;
  atc_code?: string;
  mah_name?: string;
  limit?: number;
  offset?: number;
}

export interface EmaStats {
  total_entries: number;
  entries_by_language: EmaLanguageCount[];
  entries_by_status: EmaStatusCount[];
  entries_by_therapeutic_area: EmaTherapeuticAreaCount[];
  orphan_medicines_count: number;
  last_sync_at?: string;
  last_sync_status?: string;
}

export interface EmaLanguageCount {
  language_code: string;
  count: number;
}

export interface EmaStatusCount {
  status: string;
  count: number;
}

export interface EmaTherapeuticAreaCount {
  therapeutic_area: string;
  count: number;
}

export interface EmaSyncLog {
  id: string;
  sync_started_at: string;
  sync_completed_at?: string;
  language_code?: string;
  sync_type?: string;
  record_limit?: number;
  records_fetched?: number;
  records_inserted?: number;
  records_updated?: number;
  records_skipped?: number;
  records_failed?: number;
  status: string;
  error_message?: string;
  warning_messages?: string[];
  api_response_time_ms?: number;
  processing_time_ms?: number;
  created_at: string;
}

export interface EmaSyncRequest {
  language?: string;
  limit?: number;
  sync_type?: 'full' | 'incremental' | 'by_language';
}

export interface EmaConfigInfo {
  ema_service: {
    api_base_url: string;
    default_language: string;
    default_sync_limit: number;
    batch_delay_ms: number;
    max_retries: number;
    supported_languages: string[];
  };
  service_version: string;
  api_documentation: string;
  features: {
    full_text_search: boolean;
    multi_language: boolean;
    batch_sync: boolean;
    real_time_sync: boolean;
    sync_tracking: boolean;
  };
}

export interface EmaHealthStatus {
  status: 'healthy' | 'unhealthy';
  service: string;
  version: string;
  timestamp: string;
  database: {
    status: 'connected' | 'disconnected';
  };
  last_sync?: {
    id: string;
    started_at: string;
    completed_at?: string;
    status: string;
    records_processed?: number;
  };
  features: {
    search: boolean;
    sync: boolean;
    multi_language: boolean;
  };
}

export interface EmaRefreshStatus {
  needs_refresh: boolean;
  days_threshold: number;
  timestamp: string;
}

export interface EmaCleanupResult {
  deleted_count: number;
  timestamp: string;
}