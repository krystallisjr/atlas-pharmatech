import { apiClient } from '../api-client';
import { ApiResponse } from '@/types/api';
import {
  Inquiry,
  CreateInquiryRequest,
  UpdateInquiryStatusRequest,
  Transaction,
  CreateTransactionRequest,
  CompleteTransactionRequest,
  InquiryMessage,
  CreateInquiryMessageRequest
} from '@/types/pharmaceutical';

export class MarketplaceService {
  // Create inquiry
  static async createInquiry(data: CreateInquiryRequest): Promise<Inquiry> {
    const response = await apiClient.post<Inquiry>('/api/marketplace/inquiries', data);
    return response;
  }

  // Get inquiry by ID
  static async getInquiry(id: string): Promise<Inquiry> {
    const response = await apiClient.get<Inquiry>(`/api/marketplace/inquiries/${id}`);
    return response;
  }

  // Get buyer's inquiries
  static async getBuyerInquiries(): Promise<Inquiry[]> {
    const response = await apiClient.get<Inquiry[]>('/api/marketplace/inquiries/buyer');
    return response;
  }

  // Get seller's inquiries
  static async getSellerInquiries(): Promise<Inquiry[]> {
    const response = await apiClient.get<Inquiry[]>('/api/marketplace/inquiries/seller');
    return response;
  }

  // Update inquiry status
  static async updateInquiryStatus(id: string, data: UpdateInquiryStatusRequest): Promise<Inquiry> {
    const response = await apiClient.put<Inquiry>(`/api/marketplace/inquiries/${id}/status`, data);
    return response;
  }

  // Create transaction
  static async createTransaction(data: CreateTransactionRequest): Promise<Transaction> {
    const response = await apiClient.post<Transaction>('/api/marketplace/transactions', data);
    return response;
  }

  // Get transaction by ID
  static async getTransaction(id: string): Promise<Transaction> {
    const response = await apiClient.get<Transaction>(`/api/marketplace/transactions/${id}`);
    return response;
  }

  // Get user's transactions
  static async getUserTransactions(): Promise<Transaction[]> {
    const response = await apiClient.get<Transaction[]>('/api/marketplace/transactions/my');
    return response;
  }

  // Complete transaction
  static async completeTransaction(id: string, data: CompleteTransactionRequest): Promise<Transaction> {
    const response = await apiClient.post<Transaction>(`/api/marketplace/transactions/${id}/complete`, data);
    return response;
  }

  // Cancel transaction
  static async cancelTransaction(id: string): Promise<Transaction> {
    const response = await apiClient.post<Transaction>(`/api/marketplace/transactions/${id}/cancel`);
    return response;
  }

  // INQUIRY MESSAGING METHODS

  // Send a message in an inquiry
  static async sendInquiryMessage(inquiryId: string, message: string): Promise<InquiryMessage> {
    const response = await apiClient.post<InquiryMessage>(
      `/api/marketplace/inquiries/${inquiryId}/messages`,
      { inquiry_id: inquiryId, message }
    );
    return response;
  }

  // Get all messages for an inquiry
  static async getInquiryMessages(inquiryId: string): Promise<InquiryMessage[]> {
    const response = await apiClient.get<InquiryMessage[]>(
      `/api/marketplace/inquiries/${inquiryId}/messages`
    );
    return response;
  }

  // Get message count for an inquiry
  static async getInquiryMessageCount(inquiryId: string): Promise<number> {
    const response = await apiClient.get<{ count: number }>(
      `/api/marketplace/inquiries/${inquiryId}/messages/count`
    );
    return response.count;
  }
}