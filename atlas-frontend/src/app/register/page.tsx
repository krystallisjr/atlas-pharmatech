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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useAuth } from '@/contexts/auth-context'
import { Loader2, Eye, EyeOff } from 'lucide-react'
import { OAuthButtons, OAuthDivider } from '@/components/auth/OAuthButtons'
import { AtlasLogo } from '@/components/atlas-logo'

const registerSchema = z.object({
  email: z.string().email('Please enter a valid email address'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
  company_name: z.string().min(2, 'Company name is required'),
  company_type: z.enum(['manufacturer', 'distributor', 'pharmacy', 'hospital']),
  address: z.string().min(5, 'Address is required'),
  city: z.string().min(2, 'City is required'),
  state: z.string().min(2, 'State is required'),
  zip_code: z.string().min(5, 'ZIP code is required'),
  phone: z.string().min(10, 'Phone number is required'),
  contact_person: z.string().min(2, 'Contact person is required'),
  license_number: z.string().min(5, 'License number is required'),
})

type RegisterFormData = z.infer<typeof registerSchema>

export default function RegisterPage() {
  const router = useRouter()
  const { register: registerUser, isAuthenticated, isLoading } = useAuth()
  const [showPassword, setShowPassword] = useState(false)

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors },
    setError,
  } = useForm<RegisterFormData>({
    resolver: zodResolver(registerSchema),
  })

  useEffect(() => {
    if (isAuthenticated) {
      router.push('/dashboard')
    }
  }, [isAuthenticated, router])

  const onSubmit = async (data: RegisterFormData) => {
    try {
      await registerUser(data)
      router.push('/dashboard')
    } catch (error) {
      setError('root', {
        message: error instanceof Error ? error.message : 'Registration failed',
      })
    }
  }

  const inputClassName = (hasError: boolean) =>
    `mt-1 bg-white border-gray-300 text-gray-900 placeholder:text-gray-400 ${hasError ? 'border-red-500' : ''}`

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-2xl w-full space-y-8">
        {/* Logo and Header */}
        <div className="text-center">
          <Link href="/" className="inline-flex items-center justify-center space-x-2 mb-6">
            <AtlasLogo size={40} />
            <span className="text-xl font-bold text-gray-900">Atlas PharmaTech</span>
          </Link>
          <h2 className="text-3xl font-bold tracking-tight text-gray-900">
            Create your account
          </h2>
          <p className="mt-2 text-sm text-gray-600">
            Already have an account?{' '}
            <Link href="/login" className="font-medium text-blue-600 hover:text-blue-500">
              Sign in
            </Link>
          </p>
        </div>

        <Card className="bg-white border-gray-200 shadow-lg">
          <CardHeader className="pb-4">
            <CardTitle className="text-gray-900">Register your business</CardTitle>
            <CardDescription className="text-gray-600">
              Join the pharmaceutical marketplace
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-4">
                  <div>
                    <Label htmlFor="email" className="text-gray-700">Email Address</Label>
                    <Input
                      id="email"
                      type="email"
                      placeholder="business@company.com"
                      {...register('email')}
                      className={inputClassName(!!errors.email)}
                    />
                    {errors.email && (
                      <p className="mt-1 text-sm text-red-600">{errors.email.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="password" className="text-gray-700">Password</Label>
                    <div className="relative mt-1">
                      <Input
                        id="password"
                        type={showPassword ? 'text' : 'password'}
                        placeholder="Create a strong password"
                        {...register('password')}
                        className={`pr-10 ${inputClassName(!!errors.password)}`}
                      />
                      <button
                        type="button"
                        className="absolute inset-y-0 right-0 pr-3 flex items-center text-gray-400 hover:text-gray-600"
                        onClick={() => setShowPassword(!showPassword)}
                      >
                        {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                      </button>
                    </div>
                    {errors.password && (
                      <p className="mt-1 text-sm text-red-600">{errors.password.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="company_name" className="text-gray-700">Company Name</Label>
                    <Input
                      id="company_name"
                      placeholder="Your company name"
                      {...register('company_name')}
                      className={inputClassName(!!errors.company_name)}
                    />
                    {errors.company_name && (
                      <p className="mt-1 text-sm text-red-600">{errors.company_name.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="company_type" className="text-gray-700">Company Type</Label>
                    <Select onValueChange={(value) => setValue('company_type', value as any)}>
                      <SelectTrigger className={`mt-1 bg-white border-gray-300 text-gray-900 ${errors.company_type ? 'border-red-500' : ''}`}>
                        <SelectValue placeholder="Select company type" />
                      </SelectTrigger>
                      <SelectContent className="bg-white border-gray-200">
                        <SelectItem value="manufacturer">Manufacturer</SelectItem>
                        <SelectItem value="distributor">Distributor</SelectItem>
                        <SelectItem value="pharmacy">Pharmacy</SelectItem>
                        <SelectItem value="hospital">Hospital</SelectItem>
                      </SelectContent>
                    </Select>
                    {errors.company_type && (
                      <p className="mt-1 text-sm text-red-600">{errors.company_type.message}</p>
                    )}
                  </div>
                </div>

                <div className="space-y-4">
                  <div>
                    <Label htmlFor="contact_person" className="text-gray-700">Contact Person</Label>
                    <Input
                      id="contact_person"
                      placeholder="Name of primary contact"
                      {...register('contact_person')}
                      className={inputClassName(!!errors.contact_person)}
                    />
                    {errors.contact_person && (
                      <p className="mt-1 text-sm text-red-600">{errors.contact_person.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="phone" className="text-gray-700">Phone Number</Label>
                    <Input
                      id="phone"
                      type="tel"
                      placeholder="(555) 123-4567"
                      {...register('phone')}
                      className={inputClassName(!!errors.phone)}
                    />
                    {errors.phone && (
                      <p className="mt-1 text-sm text-red-600">{errors.phone.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="license_number" className="text-gray-700">License Number</Label>
                    <Input
                      id="license_number"
                      placeholder="Pharmaceutical license number"
                      {...register('license_number')}
                      className={inputClassName(!!errors.license_number)}
                    />
                    {errors.license_number && (
                      <p className="mt-1 text-sm text-red-600">{errors.license_number.message}</p>
                    )}
                  </div>

                  <div>
                    <Label htmlFor="address" className="text-gray-700">Address</Label>
                    <Input
                      id="address"
                      placeholder="Street address"
                      {...register('address')}
                      className={inputClassName(!!errors.address)}
                    />
                    {errors.address && (
                      <p className="mt-1 text-sm text-red-600">{errors.address.message}</p>
                    )}
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <div>
                  <Label htmlFor="city" className="text-gray-700">City</Label>
                  <Input
                    id="city"
                    placeholder="City"
                    {...register('city')}
                    className={inputClassName(!!errors.city)}
                  />
                  {errors.city && (
                    <p className="mt-1 text-sm text-red-600">{errors.city.message}</p>
                  )}
                </div>

                <div>
                  <Label htmlFor="state" className="text-gray-700">State</Label>
                  <Input
                    id="state"
                    placeholder="State"
                    {...register('state')}
                    className={inputClassName(!!errors.state)}
                  />
                  {errors.state && (
                    <p className="mt-1 text-sm text-red-600">{errors.state.message}</p>
                  )}
                </div>

                <div>
                  <Label htmlFor="zip_code" className="text-gray-700">ZIP Code</Label>
                  <Input
                    id="zip_code"
                    placeholder="12345"
                    {...register('zip_code')}
                    className={inputClassName(!!errors.zip_code)}
                  />
                  {errors.zip_code && (
                    <p className="mt-1 text-sm text-red-600">{errors.zip_code.message}</p>
                  )}
                </div>
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
                    Creating Account...
                  </>
                ) : (
                  'Create Account'
                )}
              </Button>
            </form>

            {/* OAuth Section - below form */}
            <OAuthDivider text="or" />

            <OAuthButtons
              mode="register"
              onError={(error) => setError('root', { message: error })}
            />

            <div className="text-center pt-4 border-t border-gray-200">
              <p className="text-sm text-gray-600">
                Already have an account?{' '}
                <Link href="/login" className="font-medium text-blue-600 hover:text-blue-500">
                  Sign in
                </Link>
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
