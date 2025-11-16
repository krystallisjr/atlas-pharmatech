// 🔐 PRODUCTION BACKUP CODES DISPLAY
// Shows backup codes during MFA enrollment with save/print functionality

'use client'

import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { Download, Printer, Copy, CheckCircle2, AlertTriangle, Key } from 'lucide-react'
import { toast } from 'react-toastify'

interface BackupCodesDisplayProps {
  codes: string[]
  onConfirm: () => void
  isLoading?: boolean
}

export function BackupCodesDisplay({ codes, onConfirm, isLoading = false }: BackupCodesDisplayProps) {
  const [hasDownloaded, setHasDownloaded] = useState(false)
  const [confirmed, setConfirmed] = useState(false)

  const handleDownload = () => {
    const content = `Atlas Pharma - MFA Backup Codes
Generated: ${new Date().toLocaleString()}

IMPORTANT: Keep these codes safe and secure!
Each code can only be used once.

Backup Codes:
${codes.map((code, i) => `${i + 1}. ${code}`).join('\n')}

⚠️ WARNING: If you lose these codes and your authenticator device,
you will be locked out of your account.

Store these codes in a secure location:
- Password manager (recommended)
- Encrypted file
- Physical safe

DO NOT share these codes with anyone.
`
    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `atlas-pharma-backup-codes-${Date.now()}.txt`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)

    setHasDownloaded(true)
    toast.success('Backup codes downloaded successfully')
  }

  const handlePrint = () => {
    const printWindow = window.open('', '_blank')
    if (!printWindow) {
      toast.error('Please allow popups to print backup codes')
      return
    }

    const content = `
      <!DOCTYPE html>
      <html>
        <head>
          <title>Atlas Pharma - MFA Backup Codes</title>
          <style>
            body {
              font-family: 'Courier New', monospace;
              padding: 40px;
              max-width: 800px;
              margin: 0 auto;
            }
            h1 {
              font-size: 24px;
              margin-bottom: 10px;
            }
            .warning {
              background: #fff3cd;
              border: 2px solid #ffc107;
              padding: 15px;
              margin: 20px 0;
              border-radius: 5px;
            }
            .codes {
              background: #f5f5f5;
              padding: 20px;
              border-radius: 5px;
              margin: 20px 0;
            }
            .code {
              font-size: 18px;
              font-weight: bold;
              margin: 10px 0;
              letter-spacing: 2px;
            }
            .instructions {
              margin-top: 30px;
              line-height: 1.6;
            }
            @media print {
              .no-print { display: none; }
            }
          </style>
        </head>
        <body>
          <h1>🔐 Atlas Pharma - MFA Backup Codes</h1>
          <p><strong>Generated:</strong> ${new Date().toLocaleString()}</p>

          <div class="warning">
            <strong>⚠️ IMPORTANT: Keep these codes safe and secure!</strong>
            <br>Each code can only be used once.
          </div>

          <div class="codes">
            <h2>Backup Codes:</h2>
            ${codes.map((code, i) => `<div class="code">${i + 1}. ${code}</div>`).join('')}
          </div>

          <div class="instructions">
            <h3>Storage Instructions:</h3>
            <ul>
              <li>✓ Store in a password manager (recommended)</li>
              <li>✓ Keep in an encrypted file</li>
              <li>✓ Store in a physical safe</li>
              <li>✗ DO NOT share with anyone</li>
              <li>✗ DO NOT store in plaintext emails</li>
            </ul>

            <p><strong>Warning:</strong> If you lose these codes and your authenticator device, you will be locked out of your account.</p>
          </div>

          <button class="no-print" onclick="window.print()" style="margin-top: 30px; padding: 10px 20px; font-size: 16px;">
            Print
          </button>
        </body>
      </html>
    `

    printWindow.document.write(content)
    printWindow.document.close()
    setHasDownloaded(true)
  }

  const handleCopyAll = () => {
    const text = codes.join('\n')
    navigator.clipboard.writeText(text).then(() => {
      toast.success('All backup codes copied to clipboard')
      setHasDownloaded(true)
    }).catch(() => {
      toast.error('Failed to copy codes')
    })
  }

  const handleCopySingle = (code: string) => {
    navigator.clipboard.writeText(code).then(() => {
      toast.success('Code copied to clipboard')
    }).catch(() => {
      toast.error('Failed to copy code')
    })
  }

  return (
    <div className="space-y-6">
      {/* Warning Alert */}
      <Alert variant="destructive">
        <AlertTriangle className="h-5 w-5" />
        <AlertDescription className="font-semibold">
          Save these backup codes immediately! You won't be able to see them again.
        </AlertDescription>
      </Alert>

      {/* Backup Codes Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Key className="h-5 w-5" />
            Your Backup Codes
          </CardTitle>
          <CardDescription>
            Use these codes if you lose access to your authenticator device. Each code can only be used once.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Codes Grid */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 p-4 bg-gray-50 rounded-lg">
            {codes.map((code, index) => (
              <div
                key={index}
                className="flex items-center justify-between p-3 bg-white rounded border hover:border-blue-500 transition-colors group"
              >
                <div className="flex items-center gap-3">
                  <span className="text-xs font-medium text-gray-500 w-6">{index + 1}.</span>
                  <code className="text-lg font-mono font-bold tracking-wider">{code}</code>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => handleCopySingle(code)}
                  className="opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <Copy className="h-4 w-4" />
                </Button>
              </div>
            ))}
          </div>

          {/* Action Buttons */}
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={handleDownload}
              className="flex-1 min-w-[150px]"
            >
              <Download className="mr-2 h-4 w-4" />
              Download
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={handlePrint}
              className="flex-1 min-w-[150px]"
            >
              <Printer className="mr-2 h-4 w-4" />
              Print
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={handleCopyAll}
              className="flex-1 min-w-[150px]"
            >
              <Copy className="mr-2 h-4 w-4" />
              Copy All
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Instructions */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Storage Instructions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-2">
            <div className="flex items-start gap-2">
              <CheckCircle2 className="h-5 w-5 text-green-600 mt-0.5" />
              <div>
                <p className="font-medium">Recommended Storage Options:</p>
                <ul className="text-sm text-gray-600 mt-1 space-y-1 ml-4 list-disc">
                  <li>Password manager (1Password, LastPass, Bitwarden)</li>
                  <li>Encrypted note in secure cloud storage</li>
                  <li>Physical safe or locked drawer</li>
                </ul>
              </div>
            </div>

            <div className="flex items-start gap-2">
              <AlertTriangle className="h-5 w-5 text-red-600 mt-0.5" />
              <div>
                <p className="font-medium">Do NOT Store:</p>
                <ul className="text-sm text-gray-600 mt-1 space-y-1 ml-4 list-disc">
                  <li>In plaintext emails or messages</li>
                  <li>In unencrypted files on your computer</li>
                  <li>In publicly accessible locations</li>
                </ul>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Confirmation Checkbox */}
      <div className="flex items-start space-x-3 p-4 border rounded-lg bg-blue-50 border-blue-200">
        <Checkbox
          id="codes-saved"
          checked={confirmed}
          onCheckedChange={(checked) => setConfirmed(checked as boolean)}
          disabled={isLoading}
        />
        <div className="space-y-1">
          <Label
            htmlFor="codes-saved"
            className="text-sm font-medium leading-none cursor-pointer"
          >
            I have saved my backup codes in a secure location
          </Label>
          <p className="text-xs text-gray-600">
            You must confirm that you've saved these codes before continuing. They will not be shown again.
          </p>
        </div>
      </div>

      {/* Continue Button */}
      <Button
        type="button"
        onClick={onConfirm}
        disabled={!confirmed || isLoading}
        className="w-full"
        size="lg"
      >
        {isLoading ? (
          <>
            <Download className="mr-2 h-4 w-4 animate-pulse" />
            Completing Setup...
          </>
        ) : (
          <>
            Continue
          </>
        )}
      </Button>

      {!hasDownloaded && (
        <Alert>
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            Please download, print, or copy your backup codes before continuing.
          </AlertDescription>
        </Alert>
      )}
    </div>
  )
}
