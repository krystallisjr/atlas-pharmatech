'use client';

import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { User, AuthResponse } from '@/types/auth';
import { AuthService } from '@/lib/services';
import { toast } from 'react-toastify';

interface AuthContextType {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  // 🔐 MFA state
  mfaRequired: boolean;
  mfaEmail: string | null;
  mfaUserId: string | null;
  // Auth methods
  login: (email: string, password: string) => Promise<void>;
  register: (data: any) => Promise<void>;
  logout: () => void;
  refreshProfile: () => Promise<void>;
  // 🔐 MFA methods
  setMfaRequired: (email: string, userId: string) => void;
  clearMfaState: () => void;
  completeMfaLogin: (user: User, token: string) => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  // 🔐 MFA state
  const [mfaRequired, setMfaRequiredState] = useState(false);
  const [mfaEmail, setMfaEmail] = useState<string | null>(null);
  const [mfaUserId, setMfaUserId] = useState<string | null>(null);

  // Initialize auth state from localStorage
  useEffect(() => {
    const initAuth = () => {
      const { token: storedToken, user: storedUser } = AuthService.getStoredAuthData();

      if (storedToken && storedUser) {
        setToken(storedToken);
        setUser(storedUser);
      }

      setIsLoading(false);
    };

    initAuth();
  }, []);

  // Login function
  const login = async (email: string, password: string) => {
    try {
      setIsLoading(true);
      console.log('🔐 Starting login process for:', email);
      const authData = await AuthService.login({ email, password });
      console.log('✅ Login API response:', authData);

      // 🔐 PRODUCTION MFA: Check if MFA verification is required
      if ('mfa_required' in authData && authData.mfa_required) {
        console.log('🔐 MFA required - showing verification modal');
        setMfaRequiredState(true);
        setMfaEmail(authData.email);
        setMfaUserId(authData.user_id);
        toast.info('MFA verification required');
        return; // Don't complete login yet
      }

      // Normal login (no MFA or trusted device)
      setToken(authData.token);
      setUser(authData.user);
      console.log('🔄 State updated - token:', !!authData.token, 'user:', !!authData.user);

      // Store in localStorage
      AuthService.storeAuthData(authData as AuthResponse);
      console.log('💾 Data stored in localStorage');

      toast.success('Login successful!');
      console.log('🎉 Login process completed');
    } catch (error) {
      console.error('❌ Login failed:', error);
      toast.error(error instanceof Error ? error.message : 'Login failed');
      throw error;
    } finally {
      setIsLoading(false);
    }
  };

  // Register function
  const register = async (data: any) => {
    try {
      setIsLoading(true);
      const authData = await AuthService.register(data);

      AuthService.storeAuthData(authData);
      setToken(authData.token);
      setUser(authData.user);

      toast.success('Registration successful!');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Registration failed');
      throw error;
    } finally {
      setIsLoading(false);
    }
  };

  // Logout function
  const logout = () => {
    AuthService.clearAuthData();
    setToken(null);
    setUser(null);
    setMfaRequiredState(false);
    setMfaEmail(null);
    setMfaUserId(null);
    toast.success('Logged out successfully');
  };

  // 🔐 MFA Methods
  const setMfaRequired = (email: string, userId: string) => {
    setMfaRequiredState(true);
    setMfaEmail(email);
    setMfaUserId(userId);
  };

  const clearMfaState = () => {
    setMfaRequiredState(false);
    setMfaEmail(null);
    setMfaUserId(null);
  };

  const completeMfaLogin = (user: User, token: string) => {
    // Complete login after successful MFA verification
    setUser(user);
    setToken(token);
    setMfaRequiredState(false);
    setMfaEmail(null);
    setMfaUserId(null);

    // Store in localStorage
    AuthService.storeAuthData({ user, token, expires_in: 3600 });

    toast.success('Login successful!');
  };

  // Refresh user profile
  const refreshProfile = async () => {
    try {
      if (token) {
        const updatedUser = await AuthService.getProfile();
        setUser(updatedUser);

        // Update stored user data
        if (typeof window !== 'undefined') {
          localStorage.setItem('atlas_user', JSON.stringify(updatedUser));
        }
      }
    } catch (error) {
      // If profile fetch fails, token might be expired
      console.error('Failed to refresh profile:', error);
      logout();
    }
  };

  const isAuthenticated = !!token && !!user;
  console.log('🔐 AuthContext state update:', { isAuthenticated, hasUser: !!user, hasToken: !!token, isLoading, mfaRequired });

  const value: AuthContextType = {
    user,
    token,
    isAuthenticated,
    isLoading,
    // 🔐 MFA state
    mfaRequired,
    mfaEmail,
    mfaUserId,
    // Auth methods
    login,
    register,
    logout,
    refreshProfile,
    // 🔐 MFA methods
    setMfaRequired,
    clearMfaState,
    completeMfaLogin,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}