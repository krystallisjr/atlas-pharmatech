import { apiClient } from './api-client';
import {
  LoginRequest,
  RegisterRequest,
  AuthResponse,
  User
} from '@/types/auth';
import {
  Pharmaceutical,
  Inventory,
  CreateInventoryRequest,
  Inquiry,
  Transaction
} from '@/types/pharmaceutical';
import {
  GenerateDocumentRequest,
  GeneratedDocument,
  ListDocumentsParams,
  DocumentListResponse,
  AuditTrailResponse,
  VerificationResult,
  KnowledgeBaseStats
} from '@/types/regulatory';
import { PaginationParams, SearchParams, PaginatedResponse } from '@/types/api';

// Authentication API
export const authApi = {
  login: (credentials: LoginRequest) =>
    apiClient.post<AuthResponse>('/api/auth/login', credentials),

  register: (userData: RegisterRequest) =>
    apiClient.post<AuthResponse>('/api/auth/register', userData),

  getProfile: () =>
    apiClient.get<User>('/api/auth/profile'),

  updateProfile: (userData: Partial<User>) =>
    apiClient.put<User>('/api/auth/profile', userData),

  refreshToken: () =>
    apiClient.post<{ token: string }>('/api/auth/refresh'),
};

// Pharmaceutical API
export const pharmaApi = {
  getAll: (params?: PaginationParams) =>
    apiClient.get<PaginatedResponse<Pharmaceutical>>('/api/pharmaceuticals'),

  getById: (id: string) =>
    apiClient.get<Pharmaceutical>(`/api/pharmaceuticals/${id}`),

  create: (pharma: Omit<Pharmaceutical, 'id' | 'created_at' | 'updated_at'>) =>
    apiClient.post<Pharmaceutical>('/api/pharmaceuticals', pharma),

  update: (id: string, pharma: Partial<Pharmaceutical>) =>
    apiClient.put<Pharmaceutical>(`/api/pharmaceuticals/${id}`, pharma),

  delete: (id: string) =>
    apiClient.delete<void>(`/api/pharmaceuticals/${id}`),

  search: (query: string) =>
    apiClient.get<Pharmaceutical[]>(`/api/pharmaceuticals/search?q=${encodeURIComponent(query)}`),

  getByNdc: (ndc: string) =>
    apiClient.get<Pharmaceutical>(`/api/public/pharmaceuticals/${ndc}`),

  getPublicCatalog: (params?: SearchParams) =>
    apiClient.get<PaginatedResponse<Pharmaceutical>>('/api/public/pharmaceuticals', params),
};

// Inventory API
export const inventoryApi = {
  getAll: (params?: PaginationParams) =>
    apiClient.get<PaginatedResponse<Inventory>>('/api/inventory'),

  getMyInventory: (params?: PaginationParams) =>
    apiClient.get<PaginatedResponse<Inventory>>('/api/inventory/my'),

  getById: (id: string) =>
    apiClient.get<Inventory>(`/api/inventory/${id}`),

  create: (inventory: CreateInventoryRequest) =>
    apiClient.post<Inventory>('/api/inventory', inventory),

  update: (id: string, inventory: Partial<Inventory>) =>
    apiClient.put<Inventory>(`/api/inventory/${id}`, inventory),

  delete: (id: string) =>
    apiClient.delete<void>(`/api/inventory/${id}`),

  getExpiring: (days: number = 30) =>
    apiClient.get<Inventory[]>(`/api/inventory/expiring?days=${days}`),

  searchPublic: (params: SearchParams & PaginationParams) =>
    apiClient.get<PaginatedResponse<Inventory>>('/api/public/inventory/search', params),
};

// Marketplace API
export const marketplaceApi = {
  // Inquiries
  getInquiries: (params?: PaginationParams) =>
    apiClient.get<PaginatedResponse<Inquiry>>('/api/marketplace/inquiries'),

  createInquiry: (inquiry: Omit<Inquiry, 'id' | 'created_at' | 'updated_at'>) =>
    apiClient.post<Inquiry>('/api/marketplace/inquiries', inquiry),

  updateInquiry: (id: string, inquiry: Partial<Inquiry>) =>
    apiClient.put<Inquiry>(`/api/marketplace/inquiries/${id}`, inquiry),

  // Transactions
  getTransactions: (params?: PaginationParams) =>
    apiClient.get<PaginatedResponse<Transaction>>('/api/marketplace/transactions'),

  createTransaction: (transaction: Omit<Transaction, 'id' | 'created_at' | 'updated_at'>) =>
    apiClient.post<Transaction>('/api/marketplace/transactions', transaction),

  updateTransaction: (id: string, transaction: Partial<Transaction>) =>
    apiClient.put<Transaction>(`/api/marketplace/transactions/${id}`, transaction),
};

// Regulatory AI Document Generation API
export const regulatoryApi = {
  // Generate new regulatory document with AI + RAG
  generate: (request: GenerateDocumentRequest) =>
    apiClient.post<GeneratedDocument>('/api/regulatory/documents/generate', request),

  // List all regulatory documents with filtering
  list: (params?: ListDocumentsParams) =>
    apiClient.get<DocumentListResponse>('/api/regulatory/documents', params),

  // Get specific document by ID
  getById: (id: string) =>
    apiClient.get<GeneratedDocument>(`/api/regulatory/documents/${id}`),

  // Approve a document (creates second signature in blockchain)
  approve: (id: string, comments?: string) =>
    apiClient.post<{ success: boolean; approved_at: string; approved_by: string; document_id: string }>(
      `/api/regulatory/documents/${id}/approve`,
      comments ? { comments } : {}
    ),

  // Verify document signatures and blockchain integrity
  verify: (id: string) =>
    apiClient.get<VerificationResult>(`/api/regulatory/documents/${id}/verify`),

  // Get complete blockchain audit trail for a document
  getAuditTrail: (id: string) =>
    apiClient.get<AuditTrailResponse>(`/api/regulatory/documents/${id}/audit-trail`),

  // Get knowledge base statistics
  getKnowledgeBaseStats: () =>
    apiClient.get<KnowledgeBaseStats>('/api/regulatory/knowledge-base/stats'),
};