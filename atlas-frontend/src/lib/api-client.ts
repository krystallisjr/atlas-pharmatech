import axios, { AxiosInstance, AxiosError, AxiosResponse } from 'axios';
import { ApiResponse, ApiError } from '@/types/api';

class ApiClient {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080',
      timeout: 60000, // 60 seconds for AI operations
      headers: {
        'Content-Type': 'application/json',
      },
      withCredentials: true, // Send cookies with requests
      // For development with self-signed certificates
      ...(process.env.NODE_ENV === 'development' && {
        httpsAgent: typeof window === 'undefined' ? new (require('https').Agent)({
          rejectUnauthorized: false
        }) : undefined
      })
    });

    this.setupInterceptors();
  }

  private setupInterceptors() {
    // Request interceptor - add auth token
    this.client.interceptors.request.use(
      (config) => {
        if (typeof window !== 'undefined') {
          // Try localStorage first, fallback to sessionStorage for iOS Safari
          const token = localStorage.getItem('atlas_token') || sessionStorage.getItem('atlas_token');
          if (token) {
            config.headers.Authorization = `Bearer ${token}`;
          }
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor - handle errors and token refresh
    this.client.interceptors.response.use(
      (response: AxiosResponse) => response,
      async (error: AxiosError<ApiError>) => {
        const originalRequest = error.config;

        // 🚫 PRODUCTION: Handle 401 Unauthorized (token expired/blacklisted)
        if (error.response?.status === 401 && originalRequest) {
          // Token expired or blacklisted - clear local storage and redirect to login
          if (typeof window !== 'undefined') {
            localStorage.removeItem('atlas_token');
            localStorage.removeItem('atlas_user');
            sessionStorage.removeItem('mfa_pending_email');
            window.location.href = '/login';
          }
          return Promise.reject(error);
        }

        // ⚠️ PRODUCTION: Handle 429 Too Many Requests (rate limiting)
        if (error.response?.status === 429) {
          const errorMessage = error.response?.data?.error ||
            error.response?.data?.message ||
            'Too many requests. Please try again in a few moments.';

          // Extract retry-after header if available
          const retryAfter = error.response.headers['retry-after'];
          const retryMessage = retryAfter
            ? `Too many requests. Please try again in ${retryAfter} seconds.`
            : errorMessage;

          console.warn('⚠️ Rate limit exceeded:', retryMessage);
          return Promise.reject(new Error(retryMessage));
        }

        // 🔒 PRODUCTION: Handle 403 Forbidden (permission denied)
        if (error.response?.status === 403) {
          const errorMessage = error.response?.data?.error ||
            error.response?.data?.message ||
            'You do not have permission to perform this action.';
          return Promise.reject(new Error(errorMessage));
        }

        // ❌ Handle all other errors
        const errorMessage = error.response?.data?.error || error.response?.data?.message || error.message || 'An unexpected error occurred';
        return Promise.reject(new Error(errorMessage));
      }
    );
  }

  // Generic request method with proper error handling
  private async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE',
    url: string,
    data?: any,
    params?: Record<string, any>
  ): Promise<T> {
    try {
      const response = await this.client.request<T>({
        method,
        url,
        data,
        params,
      });
      return response.data;
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('An unexpected error occurred');
    }
  }

  // HTTP methods
  async get<T>(url: string, config?: { params?: Record<string, any> }): Promise<T> {
    return this.request<T>('GET', url, undefined, config?.params);
  }

  async post<T>(url: string, data?: any): Promise<T> {
    return this.request<T>('POST', url, data);
  }

  async put<T>(url: string, data?: any): Promise<T> {
    return this.request<T>('PUT', url, data);
  }

  async delete<T>(url: string): Promise<T> {
    return this.request<T>('DELETE', url);
  }

  // File upload method - uses fresh axios instance to avoid default Content-Type header
  async upload<T>(url: string, file: File, onProgress?: (progress: number) => void): Promise<T> {
    const formData = new FormData();
    formData.append('file', file);

    try {
      // Use axios directly (not the configured instance) to avoid default Content-Type header.
      // This ensures axios auto-detects FormData and sets multipart/form-data with boundary.
      const token = typeof window !== 'undefined' ? localStorage.getItem('atlas_token') : null;

      const response = await axios.post<T>(
        `${process.env.NEXT_PUBLIC_API_URL || 'https://localhost:8443'}${url}`,
        formData,
        {
          headers: {
            ...(token ? { Authorization: `Bearer ${token}` } : {}),
            // Do NOT set Content-Type - let axios auto-detect from FormData
          },
          onUploadProgress: (progressEvent) => {
            if (onProgress && progressEvent.total) {
              const progress = Math.round((progressEvent.loaded * 100) / progressEvent.total);
              onProgress(progress);
            }
          },
        }
      );
      return response.data;
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('Upload failed');
    }
  }
}

export const apiClient = new ApiClient();