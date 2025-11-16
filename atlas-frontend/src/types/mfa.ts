// 🔐 PRODUCTION MFA/TOTP TYPES

export interface MfaStatus {
  mfa_enabled: boolean;
  enrolled_at: string | null;
  backup_codes_remaining: number;
  trusted_devices_count: number;
}

export interface StartEnrollmentRequest {
  password: string; // Re-authenticate before showing secret
}

export interface StartEnrollmentResponse {
  secret: string; // TOTP secret in base32 format
  qr_code: string; // Base64-encoded PNG QR code
  backup_codes: string[]; // 10 backup codes (user must save)
}

export interface CompleteEnrollmentRequest {
  secret: string;
  code: string; // TOTP code to verify user scanned QR
  backup_codes: string[]; // Same codes from start_enrollment
  device_name?: string; // Optional device name
}

export interface VerifyMfaRequest {
  code: string; // TOTP code or backup code
  trust_device?: boolean; // Whether to add this device to trusted list
}

export interface VerifyMfaResponse {
  success: boolean;
  trusted_device_id?: string; // If trust_device was true
}

export interface DisableMfaRequest {
  password: string; // Re-authenticate before disabling
}

export interface TrustedDevice {
  id: string;
  device_name: string | null;
  device_type: string | null;
  device_fingerprint: string;
  ip_address: string | null;
  user_agent: string | null;
  trusted_at: string;
  expires_at: string;
  is_active: boolean;
  created_at: string;
}

// 🔒 MFA state for auth context
export interface MfaState {
  // Set to true when user successfully logs in but needs MFA verification
  requires_mfa: boolean;
  // Temporary token for MFA verification (not a full auth token)
  mfa_token?: string;
  // User email for MFA flow
  email?: string;
}

// 🎨 Component prop types
export interface MfaEnrollmentWizardProps {
  onComplete: () => void;
  onCancel: () => void;
}

export interface MfaVerificationModalProps {
  isOpen: boolean;
  email: string;
  onVerified: () => void;
  onCancel: () => void;
}

export interface BackupCodesDisplayProps {
  codes: string[];
  onConfirm: () => void;
}

export interface TrustedDeviceCardProps {
  device: TrustedDevice;
  onRevoke: (id: string) => void;
}
