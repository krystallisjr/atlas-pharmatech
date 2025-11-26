import { apiClient } from '../api-client';
import { ApiResponse } from '@/types/api';
import {
  User,
  LoginRequest,
  RegisterRequest,
  AuthResponse
} from '@/types/auth';

export class AuthService {
  // Register new user
  static async register(data: RegisterRequest): Promise<AuthResponse> {
    const response = await apiClient.post<AuthResponse>('/api/auth/register', data);
    return response;
  }

  // Login user
  // 🔐 PRODUCTION MFA: Returns either AuthResponse or MfaRequiredResponse
  static async login(data: LoginRequest): Promise<AuthResponse | { mfa_required: true; email: string; user_id: string }> {
    const response = await apiClient.post<any>('/api/auth/login', data);

    // 🔐 Check if MFA is required
    if (response.mfa_required) {
      console.log('🔐 MFA verification required for:', response.email);
      return {
        mfa_required: true,
        email: response.email,
        user_id: response.user_id,
      };
    }

    // Backend returns object {user, token}
    return {
      user: response.user,
      token: response.token,
      expires_in: 3600 // 1 hour in seconds (standard JWT expiration)
    };
  }

  // Refresh JWT token
  static async refreshToken(): Promise<AuthResponse> {
    const response = await apiClient.post<AuthResponse>('/api/auth/refresh');
    return response;
  }

  // Get user profile
  static async getProfile(): Promise<User> {
    const response = await apiClient.get<User>('/api/auth/profile');
    return response;
  }

  // Update user profile
  static async updateProfile(data: Partial<User>): Promise<User> {
    const response = await apiClient.put<User>('/api/auth/profile', data);
    return response;
  }

  // Delete user account
  static async deleteAccount(): Promise<void> {
    await apiClient.delete('/api/auth/delete');
  }

  // Store auth data in localStorage
  static storeAuthData(authData: AuthResponse): void {
    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem('atlas_token', authData.token);
        localStorage.setItem('atlas_user', JSON.stringify(authData.user));
        // Also store in sessionStorage as fallback for iOS Safari
        sessionStorage.setItem('atlas_token', authData.token);
        sessionStorage.setItem('atlas_user', JSON.stringify(authData.user));
      } catch (error) {
        console.error('Failed to store auth data:', error);
      }
    }
  }

  // Get stored auth data
  static getStoredAuthData(): { token: string | null; user: User | null } {
    if (typeof window === 'undefined') {
      return { token: null, user: null };
    }

    // Try localStorage first, fallback to sessionStorage for iOS Safari
    const token = localStorage.getItem('atlas_token') || sessionStorage.getItem('atlas_token');
    const userStr = localStorage.getItem('atlas_user') || sessionStorage.getItem('atlas_user');
    const user = userStr ? JSON.parse(userStr) : null;

    return { token, user };
  }

  // Clear auth data from localStorage
  static clearAuthData(): void {
    if (typeof window !== 'undefined') {
      localStorage.removeItem('atlas_token');
      localStorage.removeItem('atlas_user');
    }
  }

  // Check if user is authenticated
  static isAuthenticated(): boolean {
    const { token } = this.getStoredAuthData();
    return !!token;
  }

  // Get current user from localStorage
  static getCurrentUser(): User | null {
    const { user } = this.getStoredAuthData();
    return user;
  }
}