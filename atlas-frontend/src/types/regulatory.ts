// Types for Regulatory AI Document Generation System
// Matches backend Rust types exactly

export type DocumentType = 'COA' | 'GDP' | 'GMP';
export type DocumentStatus = 'draft' | 'approved' | 'rejected';

export interface GenerateDocumentRequest {
  document_type: DocumentType;
  product_name?: string;
  batch_number?: string;
  manufacturer?: string;
  test_results?: Record<string, any>;
  custom_fields?: Record<string, any>;
}

export interface RagContextEntry {
  regulation_source: string;
  regulation_section: string;
  section_title: string;
  content: string;
  similarity: number;
}

export interface GeneratedDocument {
  id: string;
  document_type: DocumentType;
  document_number: string;
  title: string;
  content: Record<string, any>;
  content_hash: string;
  generated_signature: string;
  approved_signature?: string;
  rag_context?: RagContextEntry[];
  status: DocumentStatus;
  generated_by: string;
  approved_by?: string;
  approved_at?: string;
  created_at: string;
  updated_at: string;
}

export interface AuditLedgerEntry {
  id: number;
  entry_id: string;
  document_id: string;
  operation: string;
  content_hash: string;
  signature: string;
  signature_public_key: string;
  signature_algorithm: string;
  previous_entry_hash?: string;
  chain_hash: string;
  metadata: Record<string, any>;
  created_at: string;
}

export interface AuditTrailResponse {
  document_id: string;
  total_entries: number;
  ledger_entries: AuditLedgerEntry[];
}

export interface VerificationResult {
  document_id: string;
  signature_valid: boolean;
  ledger_valid: boolean;
  overall_valid: boolean;
  verified_at: string;
}

export interface KnowledgeBaseStats {
  total_entries: number;
  by_document_type: Array<{
    document_type: string;
    count: number;
    unique_sources: number;
  }>;
}

export interface ListDocumentsParams {
  document_type?: DocumentType;
  status?: DocumentStatus;
  page?: number;
  page_size?: number;
}

export interface DocumentListResponse {
  documents: GeneratedDocument[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}
