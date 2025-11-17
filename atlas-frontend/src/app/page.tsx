'use client'

import Link from 'next/link'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useAuth } from '@/contexts/auth-context'
import {
  Package,
  TrendingUp,
  Shield,
  Users,
  ArrowRight,
} from 'lucide-react'

export default function HomePage() {
  const { isAuthenticated, user } = useAuth()

  if (isAuthenticated) {
    // Redirect to dashboard if already authenticated
    if (typeof window !== 'undefined') {
      window.location.href = '/dashboard'
    }
    return null
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
      {/* Header */}
      <header className="bg-white shadow-sm">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-6">
            <div className="flex items-center">
              <h1 className="text-2xl font-bold text-gray-900">Atlas PharmaTech</h1>
            </div>
            <div className="flex items-center space-x-4">
              <Link href="/login">
                <Button variant="ghost">Sign In</Button>
              </Link>
              <Link href="/register">
                <Button>Get Started</Button>
              </Link>
            </div>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16">
        <div className="text-center">
          <h1 className="text-4xl font-extrabold text-gray-900 sm:text-5xl md:text-6xl">
            Pharmaceutical Inventory
            <span className="block text-blue-600">Management & Trading</span>
          </h1>
          <p className="mt-3 max-w-md mx-auto text-base text-gray-500 sm:text-lg md:mt-5 md:text-xl md:max-w-3xl">
            Connect with pharmaceutical companies nationwide. Manage inventory, track expirations, and trade medications securely on Atlas.
          </p>
          <div className="mt-5 max-w-md mx-auto sm:flex sm:justify-center md:mt-8">
            <div className="rounded-md shadow">
              <Link href="/register">
                <Button size="lg" className="w-full">
                  Start Trading Today
                  <ArrowRight className="ml-2 h-5 w-5" />
                </Button>
              </Link>
            </div>
            <div className="mt-3 rounded-md shadow sm:mt-0 sm:ml-3">
              <Link href="/login">
                <Button variant="outline" size="lg" className="w-full">
                  Sign In
                </Button>
              </Link>
            </div>
          </div>
        </div>

        {/* Features Section */}
        <div className="mt-20">
          <div className="text-center">
            <h2 className="text-3xl font-extrabold text-gray-900">
              Built for Pharmaceutical Industry
            </h2>
            <p className="mt-4 max-w-2xl mx-auto text-xl text-gray-500">
              Professional-grade inventory management with compliance and security at its core.
            </p>
          </div>

          <div className="mt-12">
            <div className="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-4">
              <Card>
                <CardHeader>
                  <Package className="h-8 w-8 text-blue-600" />
                  <CardTitle>Inventory Management</CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    Track pharmaceutical products, monitor expiry dates, and manage stock levels with real-time updates.
                  </CardDescription>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <Users className="h-8 w-8 text-blue-600" />
                  <CardTitle>Marketplace</CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    Connect with verified pharmaceutical companies. Buy and sell medications with confidence.
                  </CardDescription>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <Shield className="h-8 w-8 text-blue-600" />
                  <CardTitle>Compliance & Security</CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    JWT authentication, AES-256 encryption, comprehensive audit trails, and role-based access control for secure pharmaceutical operations.
                  </CardDescription>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <TrendingUp className="h-8 w-8 text-blue-600" />
                  <CardTitle>Analytics & Insights</CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    Advanced reporting, expiry forecasting, and business intelligence for pharmaceutical operations.
                  </CardDescription>
                </CardContent>
              </Card>
            </div>
          </div>
        </div>

        {/* CTA Section */}
        <div className="mt-20 bg-blue-600 rounded-lg shadow-xl overflow-hidden">
          <div className="px-6 py-12 sm:px-12 sm:py-16 lg:px-16">
            <div className="text-center">
              <h2 className="text-3xl font-extrabold text-white">
                Ready to streamline your pharmaceutical operations?
              </h2>
              <p className="mt-4 text-xl text-blue-100">
                Join hundreds of pharmaceutical companies already using Atlas.
              </p>
              <div className="mt-8">
                <Link href="/register">
                  <Button size="lg" variant="secondary">
                    Get Started
                    <ArrowRight className="ml-2 h-5 w-5" />
                  </Button>
                </Link>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="bg-white">
        <div className="max-w-7xl mx-auto py-12 px-4 sm:px-6 lg:px-8">
          <div className="text-center">
            <p className="text-base text-gray-500">
              &copy; 2024 Atlas PharmaTech. Built for the pharmaceutical industry.
            </p>
          </div>
        </div>
      </footer>
    </div>
  )
}