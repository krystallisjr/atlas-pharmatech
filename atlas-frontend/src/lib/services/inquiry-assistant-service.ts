import { apiClient } from '../api-client';
import type {
  GenerateSuggestionRequest,
  InquirySuggestion,
  AcceptSuggestionRequest,
  AcceptSuggestionResponse,
  AssistantQuotaStatus,
} from '@/types/inquiry-assistant';

export class InquiryAssistantService {
  /**
   * Generate AI suggestion for an inquiry
   */
  static async generateSuggestion(
    inquiryId: string,
    request: GenerateSuggestionRequest
  ): Promise<InquirySuggestion> {
    return await apiClient.post<InquirySuggestion>(
      `/api/inquiry-assistant/inquiries/${inquiryId}/suggestions`,
      request
    );
  }

  /**
   * Get suggestion by ID
   */
  static async getSuggestion(suggestionId: string): Promise<InquirySuggestion> {
    return await apiClient.get<InquirySuggestion>(
      `/api/inquiry-assistant/suggestions/${suggestionId}`
    );
  }

  /**
   * Accept and send suggestion as message
   */
  static async acceptSuggestion(
    suggestionId: string,
    request: AcceptSuggestionRequest = {}
  ): Promise<AcceptSuggestionResponse> {
    return await apiClient.post<AcceptSuggestionResponse>(
      `/api/inquiry-assistant/suggestions/${suggestionId}/accept`,
      request
    );
  }

  /**
   * Get all suggestions for an inquiry
   */
  static async getInquirySuggestions(inquiryId: string): Promise<InquirySuggestion[]> {
    return await apiClient.get<InquirySuggestion[]>(
      `/api/inquiry-assistant/inquiries/${inquiryId}/suggestions`
    );
  }

  /**
   * Get quota status
   */
  static async getQuota(): Promise<AssistantQuotaStatus> {
    return await apiClient.get<AssistantQuotaStatus>('/api/inquiry-assistant/quota');
  }
}
