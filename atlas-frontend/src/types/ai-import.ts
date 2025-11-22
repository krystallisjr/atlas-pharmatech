// AI Import Types for Atlas PharmaTech

export interface AiImportSession {
  id: string;
  status: 'analyzing' | 'mapping_review' | 'importing' | 'completed' | 'failed';
  original_filename: string;
  file_type: string;
  detected_columns: string[];
  suggested_mapping: Record<string, string>;
  confidence_scores: Record<string, number>;
  warnings: string[];
  total_rows: number;
  rows_processed: number;
  rows_imported: number;
  rows_failed: number;
  rows_flagged: number;
  ndc_validated: number;
  ndc_not_found: number;
  auto_enriched: number;
  ai_cost_usd: string;
  progress_percentage: number;
  created_at: string;
  completed_at: string | null;
  error_message: string | null;
}

export interface AiImportRowResult {
  id: string;
  session_id: string;
  row_number: number;
  source_data: Record<string, any>;
  status: 'imported' | 'failed' | 'flagged_for_review';
  mapped_data: Record<string, any> | null;
  validation_errors: string[];
  validation_warnings: string[];
  matched_ndc: string | null;
  openfda_match_confidence: number | null;
  openfda_enriched_fields: string[] | null;
  created_inventory_id: string | null;
  created_pharmaceutical_id: string | null;
  processed_at: string;
}

export interface UserQuota {
  monthly_import_limit: number;
  monthly_imports_used: number;
  monthly_cost_limit_usd: string;
  monthly_cost_used_usd: string;
  imports_remaining: number;
  cost_remaining_usd: string;
  period_start: string;
  period_end: string;
}
