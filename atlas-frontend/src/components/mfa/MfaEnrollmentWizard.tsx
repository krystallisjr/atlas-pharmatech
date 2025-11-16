// 🔐 PRODUCTION MFA ENROLLMENT WIZARD
// Multi-step wizard for enabling MFA with QR code, backup codes, and verification

'use client'

import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Progress } from '@/components/ui/progress'
import {
  Shield,
  Smartphone,
  Key,
  CheckCircle2,
  Loader2,
  Eye,
  EyeOff,
  AlertTriangle,
} from 'lucide-react'
import { MfaService } from '@/lib/services'
import { BackupCodesDisplay } from './BackupCodesDisplay'
import { toast } from 'react-toastify'
import Image from 'next/image'

interface MfaEnrollmentWizardProps {
  onComplete: () => void
  onCancel: () => void
}

type Step = 'authenticate' | 'scan-qr' | 'backup-codes' | 'verify' | 'complete'

export function MfaEnrollmentWizard({ onComplete, onCancel }: MfaEnrollmentWizardProps) {
  const [currentStep, setCurrentStep] = useState<Step>('authenticate')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [secret, setSecret] = useState('')
  const [qrCode, setQrCode] = useState('')
  const [backupCodes, setBackupCodes] = useState<string[]>([])
  const [verificationCode, setVerificationCode] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const steps: Record<Step, { title: string; description: string; icon: any }> = {
    authenticate: {
      title: 'Verify Password',
      description: 'Re-authenticate to enable MFA',
      icon: Shield,
    },
    'scan-qr': {
      title: 'Scan QR Code',
      description: 'Add to your authenticator app',
      icon: Smartphone,
    },
    'backup-codes': {
      title: 'Save Backup Codes',
      description: 'Store these in a safe place',
      icon: Key,
    },
    verify: {
      title: 'Verify Setup',
      description: 'Confirm everything works',
      icon: CheckCircle2,
    },
    complete: {
      title: 'Complete',
      description: 'MFA is now enabled',
      icon: CheckCircle2,
    },
  }

  const stepOrder: Step[] = ['authenticate', 'scan-qr', 'backup-codes', 'verify', 'complete']
  const currentStepIndex = stepOrder.indexOf(currentStep)
  const progress = ((currentStepIndex + 1) / stepOrder.length) * 100

  // Step 1: Authenticate with password
  const handleAuthenticate = async () => {
    if (!password) {
      setError('Please enter your password')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      console.log('🔐 Starting MFA enrollment...')
      const response = await MfaService.startEnrollment(password)

      setSecret(response.secret)
      setQrCode(response.qr_code)
      setBackupCodes(response.backup_codes)

      toast.success('Authentication successful')
      setCurrentStep('scan-qr')
    } catch (err) {
      console.error('❌ Authentication failed:', err)
      const errorMessage = err instanceof Error ? err.message : 'Authentication failed'
      setError(errorMessage)
      toast.error(errorMessage)
    } finally {
      setIsLoading(false)
    }
  }

  // Step 4: Verify TOTP code and complete enrollment
  const handleVerify = async () => {
    const cleanCode = verificationCode.replace(/\s/g, '')

    if (cleanCode.length !== 6) {
      setError('Please enter a valid 6-digit code')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      console.log('✅ Verifying TOTP code and completing enrollment...')
      await MfaService.completeEnrollment(secret, cleanCode, backupCodes)

      toast.success('MFA enabled successfully!')
      setCurrentStep('complete')

      // Auto-close and refresh after 2 seconds
      setTimeout(() => {
        onComplete()
      }, 2000)
    } catch (err) {
      console.error('❌ Verification failed:', err)
      const errorMessage = err instanceof Error ? err.message : 'Verification failed'
      setError(errorMessage)
      toast.error(errorMessage)
    } finally {
      setIsLoading(false)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent, action: () => void) => {
    if (e.key === 'Enter') {
      action()
    }
  }

  const renderStep = () => {
    switch (currentStep) {
      case 'authenticate':
        return (
          <div className="space-y-6">
            <Alert>
              <Shield className="h-4 w-4" />
              <AlertDescription>
                To enable two-factor authentication, please re-enter your password for security verification.
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <div className="relative">
                <Input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  placeholder="Enter your password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  onKeyPress={(e) => handleKeyPress(e, handleAuthenticate)}
                  disabled={isLoading}
                  autoFocus
                />
                <button
                  type="button"
                  className="absolute inset-y-0 right-0 pr-3 flex items-center"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={onCancel}
                disabled={isLoading}
                className="flex-1"
              >
                Cancel
              </Button>
              <Button
                type="button"
                onClick={handleAuthenticate}
                disabled={!password || isLoading}
                className="flex-1"
              >
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Verifying...
                  </>
                ) : (
                  'Continue'
                )}
              </Button>
            </div>
          </div>
        )

      case 'scan-qr':
        return (
          <div className="space-y-6">
            <Alert>
              <Smartphone className="h-4 w-4" />
              <AlertDescription>
                Scan this QR code with your authenticator app (Google Authenticator, Authy, 1Password, etc.)
              </AlertDescription>
            </Alert>

            {/* QR Code Display */}
            <div className="flex justify-center p-6 bg-white rounded-lg border-2 border-dashed">
              {qrCode ? (
                <Image
                  src={`data:image/png;base64,${qrCode}`}
                  alt="MFA QR Code"
                  width={300}
                  height={300}
                  className="rounded"
                />
              ) : (
                <div className="w-64 h-64 bg-gray-100 animate-pulse rounded" />
              )}
            </div>

            {/* Manual Entry Option */}
            <Card>
              <CardHeader>
                <CardTitle className="text-sm">Can't scan the QR code?</CardTitle>
                <CardDescription className="text-xs">
                  Enter this code manually in your authenticator app:
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-2 p-3 bg-gray-50 rounded font-mono text-sm break-all">
                  <code className="flex-1">{secret}</code>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      navigator.clipboard.writeText(secret)
                      toast.success('Secret copied to clipboard')
                    }}
                  >
                    Copy
                  </Button>
                </div>
              </CardContent>
            </Card>

            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setCurrentStep('authenticate')}
                className="flex-1"
              >
                Back
              </Button>
              <Button
                type="button"
                onClick={() => setCurrentStep('backup-codes')}
                className="flex-1"
              >
                Continue
              </Button>
            </div>
          </div>
        )

      case 'backup-codes':
        return (
          <BackupCodesDisplay
            codes={backupCodes}
            onConfirm={() => setCurrentStep('verify')}
            isLoading={false}
          />
        )

      case 'verify':
        return (
          <div className="space-y-6">
            <Alert>
              <CheckCircle2 className="h-4 w-4" />
              <AlertDescription>
                Almost done! Enter the 6-digit code from your authenticator app to verify everything is working.
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="verification-code">Verification Code</Label>
              <Input
                id="verification-code"
                type="text"
                placeholder="000000"
                value={verificationCode}
                onChange={(e) => {
                  const value = e.target.value.replace(/\D/g, '').slice(0, 6)
                  setVerificationCode(value)
                }}
                onKeyPress={(e) => handleKeyPress(e, handleVerify)}
                className="text-center text-2xl tracking-widest font-mono"
                maxLength={6}
                autoFocus
                disabled={isLoading}
              />
              <p className="text-xs text-gray-500 text-center">
                Enter the 6-digit code from your authenticator app
              </p>
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setCurrentStep('backup-codes')}
                disabled={isLoading}
                className="flex-1"
              >
                Back
              </Button>
              <Button
                type="button"
                onClick={handleVerify}
                disabled={verificationCode.length !== 6 || isLoading}
                className="flex-1"
              >
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Verifying...
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="mr-2 h-4 w-4" />
                    Complete Setup
                  </>
                )}
              </Button>
            </div>
          </div>
        )

      case 'complete':
        return (
          <div className="space-y-6 text-center py-8">
            <div className="flex justify-center">
              <div className="w-20 h-20 bg-green-100 rounded-full flex items-center justify-center">
                <CheckCircle2 className="w-10 h-10 text-green-600" />
              </div>
            </div>

            <div>
              <h3 className="text-2xl font-bold text-gray-900">All Set!</h3>
              <p className="text-gray-600 mt-2">
                Two-factor authentication has been enabled successfully.
              </p>
            </div>

            <Alert>
              <Shield className="h-4 w-4" />
              <AlertDescription>
                Your account is now protected with an extra layer of security. You'll be asked for a verification code when logging in from new devices.
              </AlertDescription>
            </Alert>

            <Button onClick={onComplete} className="w-full" size="lg">
              Done
            </Button>
          </div>
        )
    }
  }

  const CurrentIcon = steps[currentStep].icon

  return (
    <div className="space-y-6">
      {/* Progress Bar */}
      <div className="space-y-2">
        <div className="flex items-center justify-between text-sm">
          <span className="font-medium text-gray-700">
            Step {currentStepIndex + 1} of {stepOrder.length}
          </span>
          <span className="text-gray-500">{Math.round(progress)}%</span>
        </div>
        <Progress value={progress} className="h-2" />
      </div>

      {/* Step Header */}
      <div className="text-center space-y-2">
        <div className="flex justify-center">
          <div className="w-12 h-12 bg-blue-100 rounded-full flex items-center justify-center">
            <CurrentIcon className="w-6 h-6 text-blue-600" />
          </div>
        </div>
        <div>
          <h2 className="text-2xl font-bold text-gray-900">{steps[currentStep].title}</h2>
          <p className="text-gray-600">{steps[currentStep].description}</p>
        </div>
      </div>

      {/* Step Content */}
      {renderStep()}
    </div>
  )
}
