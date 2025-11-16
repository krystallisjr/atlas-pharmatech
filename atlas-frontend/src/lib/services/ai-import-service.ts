import { apiClient } from '../api-client';
import type { AiImportSession, AiImportRowResult, UserQuota } from '@/types/ai-import';

export class AiImportService {
  /**
   * Upload a file for AI analysis
   */
  static async uploadFile(file: File): Promise<AiImportSession> {
    const formData = new FormData();
    formData.append('file', file);

    return await apiClient.post<AiImportSession>('/api/ai-import/upload', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    });
  }

  /**
   * Get session details
   */
  static async getSession(sessionId: string): Promise<AiImportSession> {
    return await apiClient.get<AiImportSession>(`/api/ai-import/session/${sessionId}`);
  }

  /**
   * Start import for a session
   */
  static async startImport(sessionId: string): Promise<AiImportSession> {
    return await apiClient.post<AiImportSession>(`/api/ai-import/session/${sessionId}/start-import`);
  }

  /**
   * List user's import sessions
   */
  static async listSessions(params?: { limit?: number; offset?: number }): Promise<AiImportSession[]> {
    return await apiClient.get<AiImportSession[]>('/api/ai-import/sessions', { params });
  }

  /**
   * Get row results for a session
   */
  static async getSessionRows(
    sessionId: string,
    params?: { limit?: number; offset?: number; status_filter?: string }
  ): Promise<AiImportRowResult[]> {
    return await apiClient.get<AiImportRowResult[]>(`/api/ai-import/session/${sessionId}/rows`, { params });
  }

  /**
   * Get user's quota
   */
  static async getUserQuota(): Promise<UserQuota> {
    return await apiClient.get<UserQuota>('/api/ai-import/quota');
  }
}
