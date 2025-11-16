// 🔐 PRODUCTION MFA SETTINGS PAGE
// Manage MFA enrollment, trusted devices, and backup codes

'use client'

import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Shield,
  ShieldCheck,
  ShieldOff,
  Smartphone,
  Key,
  Trash2,
  Plus,
  Loader2,
  Eye,
  EyeOff,
  AlertTriangle,
  Calendar,
  Monitor,
} from 'lucide-react'
import { MfaService } from '@/lib/services'
import { MfaStatus, TrustedDevice } from '@/types/mfa'
import { MfaEnrollmentWizard } from '@/components/mfa/MfaEnrollmentWizard'
import { toast } from 'react-toastify'
import { useAuth } from '@/contexts/auth-context'

export default function MfaSettingsPage() {
  const { refreshProfile } = useAuth()
  const [mfaStatus, setMfaStatus] = useState<MfaStatus | null>(null)
  const [trustedDevices, setTrustedDevices] = useState<TrustedDevice[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [showEnrollmentWizard, setShowEnrollmentWizard] = useState(false)
  const [showDisableDialog, setShowDisableDialog] = useState(false)
  const [disablePassword, setDisablePassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [isDisabling, setIsDisabling] = useState(false)

  useEffect(() => {
    loadMfaData()
  }, [])

  const loadMfaData = async () => {
    setIsLoading(true)
    try {
      const [status, devices] = await Promise.all([
        MfaService.getStatus(),
        MfaService.getTrustedDevices(),
      ])
      setMfaStatus(status)
      setTrustedDevices(devices)
    } catch (error) {
      console.error('❌ Failed to load MFA data:', error)
      toast.error('Failed to load MFA settings')
    } finally {
      setIsLoading(false)
    }
  }

  const handleEnableComplete = async () => {
    setShowEnrollmentWizard(false)
    await loadMfaData()
    await refreshProfile()
    toast.success('MFA enabled successfully!')
  }

  const handleDisableMfa = async () => {
    if (!disablePassword) {
      toast.error('Please enter your password')
      return
    }

    setIsDisabling(true)
    try {
      await MfaService.disableMfa(disablePassword)
      setShowDisableDialog(false)
      setDisablePassword('')
      await loadMfaData()
      await refreshProfile()
      toast.success('MFA disabled successfully')
    } catch (error) {
      console.error('❌ Failed to disable MFA:', error)
      toast.error(error instanceof Error ? error.message : 'Failed to disable MFA')
    } finally {
      setIsDisabling(false)
    }
  }

  const handleRevokeTrustedDevice = async (deviceId: string) => {
    if (!confirm('Are you sure you want to revoke this trusted device?')) {
      return
    }

    try {
      await MfaService.revokeTrustedDevice(deviceId)
      setTrustedDevices(devices => devices.filter(d => d.id !== deviceId))
      toast.success('Trusted device revoked')
    } catch (error) {
      console.error('❌ Failed to revoke device:', error)
      toast.error('Failed to revoke device')
    }
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  if (isLoading) {
    return (
      <div className="container mx-auto py-10 px-4 max-w-4xl">
        <div className="space-y-6">
          <Skeleton className="h-12 w-64" />
          <Skeleton className="h-40 w-full" />
          <Skeleton className="h-60 w-full" />
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto py-10 px-4 max-w-4xl">
      <div className="space-y-6">
        {/* Header */}
        <div>
          <h1 className="text-3xl font-bold flex items-center gap-3">
            <Shield className="w-8 h-8 text-blue-600" />
            Two-Factor Authentication
          </h1>
          <p className="text-gray-600 mt-2">
            Protect your account with an extra layer of security
          </p>
        </div>

        {/* MFA Status Card */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                {mfaStatus?.mfa_enabled ? (
                  <ShieldCheck className="w-6 h-6 text-green-600" />
                ) : (
                  <ShieldOff className="w-6 h-6 text-gray-400" />
                )}
                <div>
                  <CardTitle>MFA Status</CardTitle>
                  <CardDescription>
                    {mfaStatus?.mfa_enabled
                      ? 'Your account is protected with two-factor authentication'
                      : 'Two-factor authentication is not enabled'}
                  </CardDescription>
                </div>
              </div>
              <Badge
                variant={mfaStatus?.mfa_enabled ? 'default' : 'secondary'}
                className={mfaStatus?.mfa_enabled ? 'bg-green-600' : ''}
              >
                {mfaStatus?.mfa_enabled ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            {mfaStatus?.mfa_enabled ? (
              <>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                  <div className="p-4 bg-gray-50 rounded-lg">
                    <p className="text-sm text-gray-600">Enabled On</p>
                    <p className="text-lg font-semibold">
                      {mfaStatus.enrolled_at ? formatDate(mfaStatus.enrolled_at) : 'N/A'}
                    </p>
                  </div>
                  <div className="p-4 bg-gray-50 rounded-lg">
                    <p className="text-sm text-gray-600">Backup Codes</p>
                    <p className="text-lg font-semibold">
                      {mfaStatus.backup_codes_remaining} remaining
                    </p>
                  </div>
                  <div className="p-4 bg-gray-50 rounded-lg">
                    <p className="text-sm text-gray-600">Trusted Devices</p>
                    <p className="text-lg font-semibold">
                      {mfaStatus.trusted_devices_count} active
                    </p>
                  </div>
                </div>

                <Alert>
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription>
                    <strong>Important:</strong> Disabling MFA will remove the extra security layer from your account. This is not recommended.
                  </AlertDescription>
                </Alert>

                <Button
                  variant="destructive"
                  onClick={() => setShowDisableDialog(true)}
                  className="w-full sm:w-auto"
                >
                  <ShieldOff className="mr-2 h-4 w-4" />
                  Disable MFA
                </Button>
              </>
            ) : (
              <>
                <Alert>
                  <Shield className="h-4 w-4" />
                  <AlertDescription>
                    <strong>Recommended:</strong> Enable two-factor authentication to add an extra layer of security to your account. You'll use an authenticator app (like Google Authenticator or Authy) to generate verification codes.
                  </AlertDescription>
                </Alert>

                <div className="space-y-3">
                  <p className="text-sm font-medium">Benefits of MFA:</p>
                  <ul className="space-y-2 text-sm text-gray-600">
                    <li className="flex items-start gap-2">
                      <Shield className="h-4 w-4 text-blue-600 mt-0.5" />
                      <span>Protect your account even if your password is compromised</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <Smartphone className="h-4 w-4 text-blue-600 mt-0.5" />
                      <span>Use any TOTP authenticator app (Google Authenticator, Authy, 1Password, etc.)</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <Key className="h-4 w-4 text-blue-600 mt-0.5" />
                      <span>Get backup codes for emergency access</span>
                    </li>
                  </ul>
                </div>

                <Button
                  onClick={() => setShowEnrollmentWizard(true)}
                  size="lg"
                  className="w-full sm:w-auto"
                >
                  <Plus className="mr-2 h-5 w-5" />
                  Enable Two-Factor Authentication
                </Button>
              </>
            )}
          </CardContent>
        </Card>

        {/* Trusted Devices Card */}
        {mfaStatus?.mfa_enabled && (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Monitor className="w-5 h-5" />
                Trusted Devices
              </CardTitle>
              <CardDescription>
                Devices where you've chosen to skip MFA verification for 30 days
              </CardDescription>
            </CardHeader>
            <CardContent>
              {trustedDevices.length === 0 ? (
                <div className="text-center py-8 text-gray-500">
                  <Monitor className="w-12 h-12 mx-auto mb-3 text-gray-400" />
                  <p>No trusted devices</p>
                  <p className="text-sm mt-1">
                    You can mark a device as trusted during login to skip MFA verification for 30 days.
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {trustedDevices.map((device) => (
                    <div
                      key={device.id}
                      className="flex items-center justify-between p-4 border rounded-lg hover:border-blue-500 transition-colors"
                    >
                      <div className="flex items-start gap-3">
                        <Monitor className="w-5 h-5 text-gray-600 mt-1" />
                        <div>
                          <p className="font-medium">
                            {device.device_name || 'Unknown Device'}
                          </p>
                          <div className="text-sm text-gray-600 space-y-1 mt-1">
                            {device.ip_address && (
                              <p className="flex items-center gap-1">
                                <span className="text-gray-400">IP:</span>
                                {device.ip_address}
                              </p>
                            )}
                            <p className="flex items-center gap-1">
                              <Calendar className="w-3 h-3" />
                              Trusted on {formatDate(device.trusted_at)}
                            </p>
                            <p className="flex items-center gap-1">
                              <Calendar className="w-3 h-3" />
                              Expires on {formatDate(device.expires_at)}
                            </p>
                          </div>
                        </div>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRevokeTrustedDevice(device.id)}
                        className="text-red-600 hover:text-red-700 hover:bg-red-50"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        )}
      </div>

      {/* Enrollment Wizard Dialog */}
      <Dialog open={showEnrollmentWizard} onOpenChange={setShowEnrollmentWizard}>
        <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
          <MfaEnrollmentWizard
            onComplete={handleEnableComplete}
            onCancel={() => setShowEnrollmentWizard(false)}
          />
        </DialogContent>
      </Dialog>

      {/* Disable MFA Dialog */}
      <Dialog open={showDisableDialog} onOpenChange={setShowDisableDialog}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangle className="w-5 h-5 text-red-600" />
              Disable Two-Factor Authentication
            </DialogTitle>
            <DialogDescription>
              Are you sure you want to disable MFA? This will reduce your account security.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <Alert variant="destructive">
              <AlertDescription>
                <strong>Warning:</strong> Disabling MFA will remove the extra security layer from your account. You'll only need your password to log in.
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="disable-password">Confirm Your Password</Label>
              <div className="relative">
                <Input
                  id="disable-password"
                  type={showPassword ? 'text' : 'password'}
                  placeholder="Enter your password"
                  value={disablePassword}
                  onChange={(e) => setDisablePassword(e.target.value)}
                  disabled={isDisabling}
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
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setShowDisableDialog(false)
                setDisablePassword('')
              }}
              disabled={isDisabling}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={handleDisableMfa}
              disabled={!disablePassword || isDisabling}
            >
              {isDisabling ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Disabling...
                </>
              ) : (
                <>
                  <ShieldOff className="mr-2 h-4 w-4" />
                  Disable MFA
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
