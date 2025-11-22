/**
 * OAuth Service
 *
 * Handles OAuth authentication flows with Google, GitHub, and Microsoft providers.
 * Works alongside existing JWT authentication system.
 */

import { apiClient } from '../api-client';
import { AuthResponse, User } from '@/types/auth';

// OAuth Provider types
export type OAuthProvider = 'google' | 'github' | 'microsoft';

export interface OAuthProviderInfo {
  name: string;
  display_name: string;
  enabled: boolean;
  icon?: string;
}

export interface OAuthProvidersResponse {
  providers: OAuthProviderInfo[];
}

export interface OAuthStartResponse {
  auth_url: string;
  state: string;
}

export interface OAuthLinkResponse {
  success: boolean;
  provider: string;
  message: string;
}

// Provider display configuration
export const OAUTH_PROVIDER_CONFIG: Record<OAuthProvider, { displayName: string; icon: string; bgColor: string; textColor: string }> = {
  google: {
    displayName: 'Google',
    icon: '/icons/google.svg',
    bgColor: 'bg-white hover:bg-gray-50',
    textColor: 'text-gray-700',
  },
  github: {
    displayName: 'GitHub',
    icon: '/icons/github.svg',
    bgColor: 'bg-gray-900 hover:bg-gray-800',
    textColor: 'text-white',
  },
  microsoft: {
    displayName: 'Microsoft',
    icon: '/icons/microsoft.svg',
    bgColor: 'bg-blue-600 hover:bg-blue-700',
    textColor: 'text-white',
  },
};

export class OAuthService {
  /**
   * Get list of enabled OAuth providers
   */
  static async getProviders(): Promise<OAuthProvidersResponse> {
    try {
      const response = await apiClient.get<OAuthProvidersResponse>('/api/auth/oauth/providers');
      return response;
    } catch (error) {
      console.error('Failed to fetch OAuth providers:', error);
      return { providers: [] };
    }
  }

  /**
   * Start OAuth flow - redirects to provider
   * For login/register, this redirects directly to the OAuth provider
   */
  static startOAuthFlow(provider: OAuthProvider): void {
    // Build the OAuth start URL
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || 'https://localhost:8443';
    const oauthUrl = `${baseUrl}/api/auth/oauth/${provider}`;

    // Redirect to OAuth provider
    window.location.href = oauthUrl;
  }

  /**
   * Start OAuth linking flow for existing authenticated users
   * Returns auth URL to open in new window/popup
   */
  static async startLinkFlow(provider: OAuthProvider): Promise<OAuthStartResponse> {
    const response = await apiClient.post<OAuthStartResponse>(`/api/auth/oauth/link/${provider}`, {});
    return response;
  }

  /**
   * Unlink OAuth provider from account
   */
  static async unlinkProvider(provider: OAuthProvider): Promise<OAuthLinkResponse> {
    const response = await apiClient.post<OAuthLinkResponse>(`/api/auth/oauth/unlink/${provider}`, {});
    return response;
  }

  /**
   * Handle OAuth callback - extract token from URL
   * Called on the callback page after OAuth redirect
   */
  static handleCallback(): {
    success: boolean;
    token?: string;
    isNewUser?: boolean;
    error?: string;
    provider?: string;
  } {
    if (typeof window === 'undefined') {
      return { success: false, error: 'Not in browser environment' };
    }

    const params = new URLSearchParams(window.location.search);

    // Check for error
    const error = params.get('error');
    if (error) {
      return {
        success: false,
        error: error,
        provider: params.get('provider') || undefined,
      };
    }

    // Check for success token
    const token = params.get('token');
    if (token) {
      const isNewUser = params.get('new_user') === 'true';
      return {
        success: true,
        token: decodeURIComponent(token),
        isNewUser,
      };
    }

    return { success: false, error: 'No token or error in callback' };
  }

  /**
   * Complete OAuth login - store token and fetch user
   */
  static async completeOAuthLogin(token: string): Promise<{ user: User; token: string }> {
    // Store token temporarily for the profile fetch
    if (typeof window !== 'undefined') {
      localStorage.setItem('atlas_token', token);
    }

    // Fetch user profile with the new token
    const user = await apiClient.get<User>('/api/auth/profile');

    return { user, token };
  }
}
