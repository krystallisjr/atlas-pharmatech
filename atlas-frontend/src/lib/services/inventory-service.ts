import { apiClient } from '../api-client';
import { ApiResponse } from '@/types/api';
import {
  Inventory,
  CreateInventoryRequest,
  UpdateInventoryRequest,
  InventorySearchRequest,
  ExpiryAlert
} from '@/types/pharmaceutical';

export class InventoryService {
  // Add inventory item
  static async addInventory(data: CreateInventoryRequest): Promise<Inventory> {
    const response = await apiClient.post<Inventory>('/api/inventory', data);
    return response;
  }

  // Get inventory by ID
  static async getInventory(id: string): Promise<Inventory> {
    const response = await apiClient.get<Inventory>(`/api/inventory/${id}`);
    return response;
  }

  // Get user's inventory
  static async getUserInventory(): Promise<Inventory[]> {
    const response = await apiClient.get<Inventory[]>('/api/inventory/my');
    return response;
  }

  // Update inventory item
  static async updateInventory(id: string, data: UpdateInventoryRequest): Promise<Inventory> {
    const response = await apiClient.put<Inventory>(`/api/inventory/${id}`, data);
    return response;
  }

  // Delete inventory item
  static async deleteInventory(id: string): Promise<void> {
    await apiClient.delete(`/api/inventory/${id}`);
  }

  // Search marketplace inventory (public)
  static async searchMarketplaceInventory(params: InventorySearchRequest): Promise<Inventory[]> {
    const searchParams = new URLSearchParams();

    if (params.search) searchParams.append('search', params.search);
    if (params.category) searchParams.append('category', params.category);
    if (params.manufacturer) searchParams.append('manufacturer', params.manufacturer);
    if (params.min_quantity) searchParams.append('min_quantity', params.min_quantity.toString());
    if (params.max_price) searchParams.append('max_price', params.max_price);
    if (params.expiry_before) searchParams.append('expiry_before', params.expiry_before);
    if (params.available_only) searchParams.append('available_only', params.available_only.toString());
    if (params.limit) searchParams.append('limit', params.limit.toString());
    if (params.offset) searchParams.append('offset', params.offset.toString());

    const url = `/api/public/inventory/search${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
    const response = await apiClient.get<Inventory[]>(url);
    return response;
  }

  // Get expiry alerts (public)
  static async getExpiryAlerts(): Promise<ExpiryAlert[]> {
    const response = await apiClient.get<ExpiryAlert[]>('/api/public/expiry-alerts');
    return response;
  }
}