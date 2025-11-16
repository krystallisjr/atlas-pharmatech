// 🔐 PRODUCTION MFA/TOTP SERVICE
// Handles all MFA-related API calls with proper error handling and rate limiting

import { apiClient } from '../api-client';
import {
  MfaStatus,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  CompleteEnrollmentRequest,
  VerifyMfaRequest,
  VerifyMfaResponse,
  DisableMfaRequest,
  TrustedDevice,
} from '@/types/mfa';

export class MfaService {
  private static readonly MFA_BASE_URL = '/api/mfa';

  /**
   * 📊 Get MFA Status
   * Returns current MFA enrollment status, backup codes remaining, and trusted device count
   */
  static async getStatus(): Promise<MfaStatus> {
    try {
      const response = await apiClient.get<MfaStatus>(`${this.MFA_BASE_URL}/status`);
      return response;
    } catch (error) {
      console.error('❌ Failed to get MFA status:', error);
      throw error;
    }
  }

  /**
   * 🔐 Start MFA Enrollment
   * Re-authenticates user with password, then generates TOTP secret, QR code, and backup codes
   *
   * @param password - User's current password for re-authentication
   * @returns Secret, QR code (base64 PNG), and 10 backup codes
   */
  static async startEnrollment(password: string): Promise<StartEnrollmentResponse> {
    try {
      console.log('🔐 Starting MFA enrollment...');
      const response = await apiClient.post<StartEnrollmentResponse>(
        `${this.MFA_BASE_URL}/enroll/start`,
        { password } as StartEnrollmentRequest
      );
      console.log('✅ MFA enrollment started successfully');
      return response;
    } catch (error) {
      console.error('❌ Failed to start MFA enrollment:', error);
      throw error;
    }
  }

  /**
   * ✅ Complete MFA Enrollment
   * Verifies TOTP code, saves encrypted secret and backup codes to database
   *
   * @param secret - TOTP secret from startEnrollment
   * @param code - 6-digit TOTP code from authenticator app
   * @param backupCodes - 10 backup codes from startEnrollment
   * @param deviceName - Optional device name for first trusted device
   */
  static async completeEnrollment(
    secret: string,
    code: string,
    backupCodes: string[],
    deviceName?: string
  ): Promise<void> {
    try {
      console.log('✅ Completing MFA enrollment...');
      await apiClient.post<void>(
        `${this.MFA_BASE_URL}/enroll/complete`,
        {
          secret,
          code,
          backup_codes: backupCodes,
          device_name: deviceName,
        } as CompleteEnrollmentRequest
      );
      console.log('🎉 MFA enrollment completed successfully');
    } catch (error) {
      console.error('❌ Failed to complete MFA enrollment:', error);
      throw error;
    }
  }

  /**
   * 🔑 Verify MFA Code
   * Verifies TOTP code or backup code, optionally adds device to trusted list
   *
   * @param code - 6-digit TOTP code or 8-character backup code
   * @param trustDevice - Whether to add this device to trusted list (30-day expiry)
   * @returns Success status and optional trusted device ID
   */
  static async verifyMfa(code: string, trustDevice: boolean = false): Promise<VerifyMfaResponse> {
    try {
      console.log('🔑 Verifying MFA code...');
      const response = await apiClient.post<VerifyMfaResponse>(
        `${this.MFA_BASE_URL}/verify`,
        {
          code,
          trust_device: trustDevice,
        } as VerifyMfaRequest
      );
      console.log('✅ MFA verification successful');
      return response;
    } catch (error) {
      console.error('❌ MFA verification failed:', error);
      throw error;
    }
  }

  /**
   * 🚫 Disable MFA
   * Re-authenticates with password, then disables MFA for the user
   *
   * @param password - User's current password for re-authentication
   */
  static async disableMfa(password: string): Promise<void> {
    try {
      console.log('🚫 Disabling MFA...');
      await apiClient.post<void>(
        `${this.MFA_BASE_URL}/disable`,
        { password } as DisableMfaRequest
      );
      console.log('✅ MFA disabled successfully');
    } catch (error) {
      console.error('❌ Failed to disable MFA:', error);
      throw error;
    }
  }

  /**
   * 📱 Get Trusted Devices
   * Returns list of all trusted devices for the current user
   */
  static async getTrustedDevices(): Promise<TrustedDevice[]> {
    try {
      const response = await apiClient.get<TrustedDevice[]>(`${this.MFA_BASE_URL}/trusted-devices`);
      return response;
    } catch (error) {
      console.error('❌ Failed to get trusted devices:', error);
      throw error;
    }
  }

  /**
   * 🗑️ Revoke Trusted Device
   * Removes a device from the trusted device list
   *
   * @param deviceId - UUID of the trusted device to revoke
   */
  static async revokeTrustedDevice(deviceId: string): Promise<void> {
    try {
      console.log(`🗑️ Revoking trusted device: ${deviceId}`);
      await apiClient.delete<void>(`${this.MFA_BASE_URL}/trusted-devices/${deviceId}`);
      console.log('✅ Trusted device revoked successfully');
    } catch (error) {
      console.error('❌ Failed to revoke trusted device:', error);
      throw error;
    }
  }

  /**
   * 💾 Store MFA temporary state
   * Used during login flow when MFA verification is required
   */
  static storeMfaState(email: string): void {
    if (typeof window !== 'undefined') {
      sessionStorage.setItem('mfa_pending_email', email);
    }
  }

  /**
   * 📖 Get MFA temporary state
   */
  static getMfaState(): string | null {
    if (typeof window === 'undefined') return null;
    return sessionStorage.getItem('mfa_pending_email');
  }

  /**
   * 🗑️ Clear MFA temporary state
   */
  static clearMfaState(): void {
    if (typeof window !== 'undefined') {
      sessionStorage.removeItem('mfa_pending_email');
    }
  }
}
