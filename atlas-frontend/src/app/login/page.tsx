'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useAuth } from '@/contexts/auth-context'
import { Loader2, Eye, EyeOff } from 'lucide-react'
import { MfaVerificationModal } from '@/components/mfa/MfaVerificationModal'
import { OAuthButtons, OAuthDivider } from '@/components/auth/OAuthButtons'
import { AtlasLogo } from '@/components/atlas-logo'

const loginSchema = z.object({
  email: z.string().email('Please enter a valid email address'),
  password: z.string().min(6, 'Password must be at least 6 characters'),
})

type LoginFormData = z.infer<typeof loginSchema>

export default function LoginPage() {
  const router = useRouter()
  const { login, isAuthenticated, isLoading, mfaRequired, mfaEmail, clearMfaState } = useAuth()
  const [showPassword, setShowPassword] = useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
    setError,
  } = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
  })

  useEffect(() => {
    if (isAuthenticated) {
      router.push('/dashboard')
    }
  }, [isAuthenticated, router])

  const onSubmit = async (data: LoginFormData) => {
    try {
      await login(data.email, data.password)
      setTimeout(() => {
        router.push('/dashboard')
      }, 100)
    } catch (error) {
      setError('root', {
        message: error instanceof Error ? error.message : 'Login failed',
      })
    }
  }

  return (
    <>
      <MfaVerificationModal
        isOpen={mfaRequired}
        email={mfaEmail || ''}
        onCancel={() => {
          clearMfaState()
          router.push('/login')
        }}
      />

      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          {/* Logo and Header */}
          <div className="text-center">
            <Link href="/" className="inline-flex items-center justify-center space-x-2 mb-6">
              <AtlasLogo size={40} />
              <span className="text-xl font-bold text-gray-900">Atlas PharmaTech</span>
            </Link>
            <h2 className="text-3xl font-bold tracking-tight text-gray-900">
              Welcome back
            </h2>
            <p className="mt-2 text-sm text-gray-600">
              Don't have an account?{' '}
              <Link href="/register" className="font-medium text-blue-600 hover:text-blue-500">
                Sign up
              </Link>
            </p>
          </div>

          <Card className="bg-white border-gray-200 shadow-lg">
            <CardHeader className="pb-4">
              <CardTitle className="text-gray-900">Sign in</CardTitle>
              <CardDescription className="text-gray-600">
                Enter your credentials to access your account
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
                <div>
                  <Label htmlFor="email" className="text-gray-700">
                    Email Address
                  </Label>
                  <Input
                    id="email"
                    type="email"
                    placeholder="you@company.com"
                    {...register('email')}
                    className={`mt-1 bg-white border-gray-300 text-gray-900 placeholder:text-gray-400 ${errors.email ? 'border-red-500' : ''}`}
                  />
                  {errors.email && (
                    <p className="mt-1 text-sm text-red-600">{errors.email.message}</p>
                  )}
                </div>

                <div>
                  <Label htmlFor="password" className="text-gray-700">
                    Password
                  </Label>
                  <div className="relative mt-1">
                    <Input
                      id="password"
                      type={showPassword ? 'text' : 'password'}
                      placeholder="Enter your password"
                      {...register('password')}
                      className={`pr-10 bg-white border-gray-300 text-gray-900 placeholder:text-gray-400 ${errors.password ? 'border-red-500' : ''}`}
                    />
                    <button
                      type="button"
                      className="absolute inset-y-0 right-0 pr-3 flex items-center text-gray-400 hover:text-gray-600"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-4 w-4" />
                      ) : (
                        <Eye className="h-4 w-4" />
                      )}
                    </button>
                  </div>
                  {errors.password && (
                    <p className="mt-1 text-sm text-red-600">{errors.password.message}</p>
                  )}
                </div>

                {errors.root && (
                  <div className="rounded-md bg-red-50 p-4 border border-red-200">
                    <p className="text-sm text-red-800">{errors.root.message}</p>
                  </div>
                )}

                <Button
                  type="submit"
                  className="w-full"
                  disabled={isLoading}
                >
                  {isLoading ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Signing in...
                    </>
                  ) : (
                    'Sign in'
                  )}
                </Button>
              </form>

              {/* OAuth Section - below email/password */}
              <OAuthDivider text="or" />

              <OAuthButtons
                mode="login"
                onError={(error) => setError('root', { message: error })}
              />

              <div className="text-center pt-4 border-t border-gray-200">
                <p className="text-sm text-gray-600">
                  New to Atlas?{' '}
                  <Link href="/register" className="font-medium text-blue-600 hover:text-blue-500">
                    Create an account
                  </Link>
                </p>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </>
  )
}
