import { User } from './auth';

// PHARMACEUTICAL TYPES
export interface Pharmaceutical {
  id: string;
  brand_name: string;
  generic_name: string;
  ndc_code?: string;
  manufacturer: string;
  category?: string;
  description?: string;
  strength?: string;
  dosage_form?: string;
  storage_requirements?: string;
  created_at: string;
}

export interface CreatePharmaceuticalRequest {
  brand_name: string;
  generic_name: string;
  ndc_code?: string;
  manufacturer: string;
  category?: string;
  description?: string;
  strength?: string;
  dosage_form?: string;
  storage_requirements?: string;
}

export interface PharmaceuticalSearchRequest {
  search?: string;
  manufacturer?: string;
  category?: string;
  ndc_code?: string;
  limit?: number;
  offset?: number;
}

// INVENTORY TYPES
export interface Inventory {
  id: string;
  user_id: string;
  pharmaceutical_id: string;
  batch_number: string;
  quantity: number;
  expiry_date: string;
  unit_price: string;
  storage_location?: string;
  status: 'available' | 'reserved' | 'sold' | 'expired';
  created_at: string;
  updated_at: string;
  days_to_expiry?: number;

  // Joined fields from backend
  pharmaceutical?: Pharmaceutical;
  seller?: User;  // Changed from 'user' to 'seller' to match backend response
}

export interface CreateInventoryRequest {
  pharmaceutical_id: string;
  batch_number: string;
  quantity: number;
  expiry_date: string;
  unit_price: string | null;
  storage_location?: string | null;
}

export interface UpdateInventoryRequest {
  quantity?: number;
  unit_price?: string;
  storage_location?: string;
  status?: 'available' | 'reserved' | 'sold' | 'expired';
}

export interface InventorySearchRequest {
  search?: string;
  category?: string;
  manufacturer?: string;
  min_quantity?: number;
  max_price?: string;
  expiry_before?: string;
  available_only?: boolean;
  limit?: number;
  offset?: number;
}

// MARKETPLACE TYPES
export interface Inquiry {
  id: string;
  inventory_id: string;
  buyer_id: string;
  quantity_requested: number;
  message?: string;
  status: 'pending' | 'negotiating' | 'accepted' | 'rejected' | 'converted_to_transaction';
  created_at: string;
  updated_at: string;
  last_message_at?: string;

  // Joined fields from backend
  inventory?: Inventory;
  buyer?: User;
  seller?: User;
}

export interface CreateInquiryRequest {
  inventory_id: string;
  quantity_requested: number;
  message?: string;
}

export interface UpdateInquiryStatusRequest {
  status: 'accepted' | 'rejected' | 'completed';
}

// INQUIRY MESSAGING TYPES
export interface InquiryMessage {
  id: string;
  inquiry_id: string;
  sender_id: string;
  sender_company: string;
  message: string;
  created_at: string;
}

export interface CreateInquiryMessageRequest {
  inquiry_id: string;
  message: string;
}

export interface Transaction {
  id: string;
  inquiry_id: string;
  seller_id: string;
  buyer_id: string;
  quantity: number;
  unit_price: string;
  total_price: string;
  transaction_date: string;
  status: 'pending' | 'completed' | 'cancelled';
}

export interface CreateTransactionRequest {
  inquiry_id: string;
}

export interface CompleteTransactionRequest {
  inquiry_id: string;
}

// AUDIT TYPES
export interface InventoryAudit {
  id: string;
  inventory_id: string;
  user_id: string;
  action: string;
  old_quantity?: number;
  new_quantity?: number;
  old_status?: string;
  new_status?: string;
  timestamp: string;
  notes?: string;

  // Joined fields
  user?: User;
}

// SEARCH AND FILTER TYPES
export interface Manufacturer {
  manufacturer: string;
  count: number;
}

export interface Category {
  category: string;
  count: number;
}

export interface ExpiryAlert {
  id: string;
  pharmaceutical_id: string;
  brand_name: string;
  generic_name: string;
  manufacturer: string;
  total_quantity: number;
  nearest_expiry: string;
  days_until_expiry: number;
  sellers: number;
}

// Export common status options
export const INVENTORY_STATUS = {
  AVAILABLE: 'available',
  RESERVED: 'reserved',
  SOLD: 'sold',
  EXPIRED: 'expired'
} as const;

export const INQUIRY_STATUS = {
  PENDING: 'pending',
  ACCEPTED: 'accepted',
  REJECTED: 'rejected',
  COMPLETED: 'completed'
} as const;

export const TRANSACTION_STATUS = {
  PENDING: 'pending',
  COMPLETED: 'completed',
  CANCELLED: 'cancelled'
} as const;