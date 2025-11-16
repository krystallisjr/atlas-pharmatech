import { apiClient } from '../api-client';
import { ApiResponse } from '@/types/api';
import {
  Pharmaceutical,
  CreatePharmaceuticalRequest,
  PharmaceuticalSearchRequest,
  Manufacturer,
  Category
} from '@/types/pharmaceutical';

export class PharmaceuticalService {
  // Create new pharmaceutical (verified users only)
  static async createPharmaceutical(data: CreatePharmaceuticalRequest): Promise<Pharmaceutical> {
    const response = await apiClient.post<Pharmaceutical>('/api/pharmaceuticals', data);
    return response;
  }

  // Get pharmaceutical by ID
  static async getPharmaceutical(id: string): Promise<Pharmaceutical> {
    const response = await apiClient.get<Pharmaceutical>(`/api/pharmaceuticals/${id}`);
    return response;
  }

  // Search pharmaceuticals
  static async searchPharmaceuticals(params: PharmaceuticalSearchRequest): Promise<Pharmaceutical[]> {
    const searchParams = new URLSearchParams();

    if (params.search) searchParams.append('search', params.search);
    if (params.manufacturer) searchParams.append('manufacturer', params.manufacturer);
    if (params.category) searchParams.append('category', params.category);
    if (params.ndc_code) searchParams.append('ndc_code', params.ndc_code);
    if (params.limit) searchParams.append('limit', params.limit.toString());
    if (params.offset) searchParams.append('offset', params.offset.toString());

    const url = `/api/pharmaceuticals/search${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
    const response = await apiClient.get<Pharmaceutical[]>(url);
    return response;
  }

  // Get all manufacturers
  static async getManufacturers(): Promise<Manufacturer[]> {
    const response = await apiClient.get<Manufacturer[]>('/api/pharmaceuticals/manufacturers');
    return response;
  }

  // Get all categories
  static async getCategories(): Promise<Category[]> {
    const response = await apiClient.get<Category[]>('/api/pharmaceuticals/categories');
    return response;
  }
}