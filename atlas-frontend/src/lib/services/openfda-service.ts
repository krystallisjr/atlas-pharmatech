import { apiClient } from '../api-client';

export interface OpenFdaDrug {
  id: string;
  product_ndc: string;
  brand_name: string;
  generic_name: string;
  labeler_name: string;
  dosage_form?: string;
  strength?: string;
  route?: string[];
  marketing_category?: string;
  dea_schedule?: string;
}

export interface OpenFdaSearchParams {
  query?: string;
  limit?: number;
  offset?: number;
}

export interface OpenFdaStats {
  total_entries: number;
  last_sync_at?: string;
  last_sync_records_fetched?: number;
  last_sync_records_inserted?: number;
  last_sync_records_updated?: number;
}

export class OpenFdaService {
  /**
   * Search OpenFDA pharmaceutical catalog
   * Used for autocomplete and drug selection
   */
  static async search(params: OpenFdaSearchParams): Promise<OpenFdaDrug[]> {
    const searchParams = new URLSearchParams();

    if (params.query) searchParams.append('query', params.query);
    if (params.limit) searchParams.append('limit', params.limit.toString());
    if (params.offset) searchParams.append('offset', params.offset.toString());

    const url = `/api/openfda/search${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
    const response = await apiClient.get<OpenFdaDrug[]>(url);
    return response;
  }

  /**
   * Get drug by NDC code
   */
  static async getByNdc(ndc: string): Promise<OpenFdaDrug | null> {
    const response = await apiClient.get<OpenFdaDrug | null>(`/api/openfda/ndc/${ndc}`);
    return response;
  }

  /**
   * Get catalog statistics
   */
  static async getStats(): Promise<OpenFdaStats> {
    const response = await apiClient.get<OpenFdaStats>('/api/openfda/stats');
    return response;
  }

  /**
   * Trigger sync from OpenFDA API (admin only)
   */
  static async triggerSync(limit?: number): Promise<any> {
    const params = limit ? `?limit=${limit}` : '';
    const response = await apiClient.post<any>(`/api/openfda/sync${params}`, {});
    return response;
  }
}
