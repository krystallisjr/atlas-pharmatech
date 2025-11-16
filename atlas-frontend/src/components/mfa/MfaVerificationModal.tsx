// 🔐 PRODUCTION MFA VERIFICATION MODAL
// Shown during login when user has MFA enabled

'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Loader2, Shield, Key, Smartphone } from 'lucide-react'
import { MfaService } from '@/lib/services'
import { useAuth } from '@/contexts/auth-context'
import { toast } from 'react-toastify'

interface MfaVerificationModalProps {
  isOpen: boolean
  email: string
  onCancel: () => void
}

export function MfaVerificationModal({ isOpen, email, onCancel }: MfaVerificationModalProps) {
  const router = useRouter()
  const { completeMfaLogin, clearMfaState } = useAuth()

  const [code, setCode] = useState('')
  const [trustDevice, setTrustDevice] = useState(false)
  const [isVerifying, setIsVerifying] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showBackupCodeHint, setShowBackupCodeHint] = useState(false)
  const [attemptCount, setAttemptCount] = useState(0)

  // Reset state when modal opens
  useEffect(() => {
    if (isOpen) {
      setCode('')
      setTrustDevice(false)
      setError(null)
      setShowBackupCodeHint(false)
      setAttemptCount(0)
    }
  }, [isOpen])

  // Auto-format TOTP code (add dashes for backup codes)
  const handleCodeChange = (value: string) => {
    // Remove all non-alphanumeric characters
    const cleaned = value.replace(/[^A-Z0-9]/gi, '').toUpperCase()

    // If it's 8 characters (backup code), format with dash: XXXX-XXXX
    if (cleaned.length <= 8) {
      const formatted = cleaned.length > 4
        ? `${cleaned.slice(0, 4)}-${cleaned.slice(4)}`
        : cleaned
      setCode(formatted)
    } else if (cleaned.length <= 6) {
      // TOTP code - just numbers
      setCode(cleaned)
    }
  }

  const handleVerify = async () => {
    // Validate code format
    const cleanCode = code.replace(/-/g, '')

    if (cleanCode.length !== 6 && cleanCode.length !== 8) {
      setError('Please enter a valid 6-digit code or 8-character backup code')
      return
    }

    setIsVerifying(true)
    setError(null)

    try {
      console.log('🔐 Verifying MFA code...')
      const result = await MfaService.verifyMfa(cleanCode, trustDevice)

      if (result.success) {
        console.log('✅ MFA verification successful')

        // Get user profile to complete login
        const { AuthService } = await import('@/lib/services')
        const user = await AuthService.getProfile()
        const { token } = AuthService.getStoredAuthData()

        if (token) {
          completeMfaLogin(user, token)
          clearMfaState()
          toast.success('Successfully verified!')

          // Redirect to dashboard
          setTimeout(() => {
            router.push('/dashboard')
          }, 100)
        } else {
          throw new Error('No auth token found')
        }
      }
    } catch (err) {
      console.error('❌ MFA verification failed:', err)
      const errorMessage = err instanceof Error ? err.message : 'Verification failed'

      setAttemptCount(prev => prev + 1)

      // Show backup code hint after 2 failed attempts
      if (attemptCount >= 1) {
        setShowBackupCodeHint(true)
      }

      // Check for rate limiting
      if (errorMessage.includes('Too many')) {
        setError(errorMessage)
        toast.error('Too many failed attempts. Please wait before trying again.')
      } else {
        setError('Invalid verification code. Please try again.')
      }
    } finally {
      setIsVerifying(false)
    }
  }

  const handleCancel = () => {
    clearMfaState()
    onCancel()
  }

  // Submit on Enter key
  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && code.length >= 6 && !isVerifying) {
      handleVerify()
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={handleCancel}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center justify-center w-12 h-12 mx-auto mb-4 rounded-full bg-blue-100">
            <Shield className="w-6 h-6 text-blue-600" />
          </div>
          <DialogTitle className="text-center text-2xl">Two-Factor Authentication</DialogTitle>
          <DialogDescription className="text-center">
            Enter the verification code from your authenticator app
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Email display */}
          <div className="text-center">
            <p className="text-sm text-gray-600">
              Signing in as: <span className="font-semibold text-gray-900">{email}</span>
            </p>
          </div>

          {/* Code input */}
          <div className="space-y-2">
            <Label htmlFor="mfa-code" className="flex items-center gap-2">
              <Smartphone className="w-4 h-4" />
              Verification Code
            </Label>
            <Input
              id="mfa-code"
              type="text"
              placeholder="000000"
              value={code}
              onChange={(e) => handleCodeChange(e.target.value)}
              onKeyPress={handleKeyPress}
              className="text-center text-2xl tracking-widest font-mono"
              maxLength={9} // 6 digits or 8 chars + 1 dash
              autoFocus
              disabled={isVerifying}
            />
            <p className="text-xs text-gray-500 text-center">
              Enter 6-digit code from your authenticator app
            </p>
          </div>

          {/* Backup code hint */}
          {showBackupCodeHint && (
            <Alert>
              <Key className="h-4 w-4" />
              <AlertDescription>
                <strong>Lost access to your authenticator?</strong>
                <br />
                You can use one of your 8-character backup codes instead.
              </AlertDescription>
            </Alert>
          )}

          {/* Error message */}
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {/* Trust device checkbox */}
          <div className="flex items-start space-x-2 p-4 bg-gray-50 rounded-lg">
            <Checkbox
              id="trust-device"
              checked={trustDevice}
              onCheckedChange={(checked) => setTrustDevice(checked as boolean)}
              disabled={isVerifying}
            />
            <div className="space-y-1">
              <Label
                htmlFor="trust-device"
                className="text-sm font-medium leading-none cursor-pointer"
              >
                Remember this device for 30 days
              </Label>
              <p className="text-xs text-gray-500">
                You won't need to verify on this device for the next month
              </p>
            </div>
          </div>
        </div>

        <DialogFooter className="flex-col sm:flex-row gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            disabled={isVerifying}
            className="w-full sm:w-auto"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleVerify}
            disabled={isVerifying || code.replace(/-/g, '').length < 6}
            className="w-full sm:w-auto"
          >
            {isVerifying ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Verifying...
              </>
            ) : (
              <>
                <Shield className="mr-2 h-4 w-4" />
                Verify
              </>
            )}
          </Button>
        </DialogFooter>

        {/* Help text */}
        <div className="text-center pt-4 border-t">
          <p className="text-xs text-gray-500">
            Having trouble? Contact support for assistance
          </p>
        </div>
      </DialogContent>
    </Dialog>
  )
}
