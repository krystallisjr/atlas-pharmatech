export interface User {
  id: string;
  email: string;
  company_name: string;
  contact_person: string;
  phone?: string;
  address?: string;
  license_number?: string;
  is_verified: boolean;
  created_at: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
  company_name: string;
  contact_person: string;
  phone?: string;
  address?: string;
  license_number?: string;
}

export interface AuthResponse {
  user: User;
  token: string;
  expires_in: number;
}

// 🔐 MFA Required Response (when user has MFA enabled)
export interface MfaRequiredResponse {
  mfa_required: true;
  email: string;
  user_id: string;
}

export interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}