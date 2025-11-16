import { apiClient } from '../api-client';
import type {
  NlQueryRequest,
  QueryResponse,
  QueryHistoryItem,
  SaveFavoriteRequest,
  FavoriteQuery,
  QuotaStatus,
} from '@/types/nl-query';

export class NlQueryService {
  /**
   * Execute a natural language query
   */
  static async executeQuery(query: string): Promise<QueryResponse> {
    return await apiClient.post<QueryResponse>('/api/nl-query/execute', { query });
  }

  /**
   * Get query session by ID
   */
  static async getSession(sessionId: string): Promise<QueryResponse> {
    return await apiClient.get<QueryResponse>(`/api/nl-query/session/${sessionId}`);
  }

  /**
   * Get query history
   */
  static async getHistory(): Promise<QueryHistoryItem[]> {
    return await apiClient.get<QueryHistoryItem[]>('/api/nl-query/history');
  }

  /**
   * Save query as favorite
   */
  static async saveFavorite(request: SaveFavoriteRequest): Promise<FavoriteQuery> {
    return await apiClient.post<FavoriteQuery>('/api/nl-query/favorites', request);
  }

  /**
   * Get favorite queries
   */
  static async getFavorites(): Promise<FavoriteQuery[]> {
    return await apiClient.get<FavoriteQuery[]>('/api/nl-query/favorites');
  }

  /**
   * Get quota status
   */
  static async getQuota(): Promise<QuotaStatus> {
    return await apiClient.get<QuotaStatus>('/api/nl-query/quota');
  }
}
