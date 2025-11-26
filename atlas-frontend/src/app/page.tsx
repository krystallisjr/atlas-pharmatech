'use client'

import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useAuth } from '@/contexts/auth-context'
import { AtlasLogo } from '@/components/atlas-logo'
import {
  Package,
  TrendingUp,
  Shield,
  Users,
  ArrowRight,
} from 'lucide-react'

export default function HomePage() {
  const { isAuthenticated } = useAuth()
  const router = useRouter()

  // Handle redirect AFTER hydration to avoid mismatch
  useEffect(() => {
    if (isAuthenticated) {
      router.push('/dashboard')
    }
  }, [isAuthenticated, router])

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
      {/* Header */}
      <header className="bg-white/80 backdrop-blur-sm shadow-sm sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center space-x-3">
              <AtlasLogo size={36} className="flex-shrink-0" />
              <span className="text-xl font-bold tracking-tight text-gray-900">
                Atlas PharmaTech
              </span>
            </div>
            <div className="flex items-center space-x-3">
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
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-16 pb-16">
        <div className="text-center">
          <h1 className="text-4xl font-bold tracking-tight leading-tight text-gray-900 sm:text-5xl md:text-6xl">
            <span className="block text-blue-600">AI-Powered</span>
            Pharmaceutical Inventory
            <span className="block text-blue-600 mt-1">
              Management & Trading
            </span>
          </h1>
          <p className="mt-6 max-w-2xl mx-auto text-lg text-gray-600 leading-relaxed">
            Connect with pharmaceutical companies nationwide. Manage inventory, track expirations, and trade medications securely on Atlas.
          </p>
          <div className="mt-8 flex flex-col sm:flex-row justify-center gap-4">
            <Link href="/register">
              <Button size="lg" className="w-full sm:w-auto px-8">
                Start Trading Today
                <ArrowRight className="ml-2 h-5 w-5" />
              </Button>
            </Link>
            <Link href="/login">
              <Button variant="outline" size="lg" className="w-full sm:w-auto px-8">
                Sign In
              </Button>
            </Link>
          </div>
        </div>

        {/* Features Section */}
        <div className="mt-24">
          <div className="text-center">
            <h2 className="text-3xl font-bold tracking-tight text-gray-900">
              Built for the Pharmaceutical Industry
            </h2>
            <p className="mt-4 max-w-2xl mx-auto text-lg text-gray-600">
              Professional-grade inventory management with compliance and security at its core.
            </p>
          </div>

          <div className="mt-12 grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
            <Card className="bg-white border-gray-200 shadow-sm hover:shadow-md transition-shadow">
              <CardHeader className="pb-2">
                <Package className="h-8 w-8 text-blue-600 mb-2" />
                <CardTitle className="text-gray-900">Inventory Management</CardTitle>
              </CardHeader>
              <CardContent>
                <CardDescription className="text-gray-600">
                  Track pharmaceutical products, monitor expiry dates, and manage stock levels with real-time updates.
                </CardDescription>
              </CardContent>
            </Card>

            <Card className="bg-white border-gray-200 shadow-sm hover:shadow-md transition-shadow">
              <CardHeader className="pb-2">
                <Users className="h-8 w-8 text-blue-600 mb-2" />
                <CardTitle className="text-gray-900">Marketplace</CardTitle>
              </CardHeader>
              <CardContent>
                <CardDescription className="text-gray-600">
                  Connect with verified pharmaceutical companies. Buy and sell medications with confidence.
                </CardDescription>
              </CardContent>
            </Card>

            <Card className="bg-white border-gray-200 shadow-sm hover:shadow-md transition-shadow">
              <CardHeader className="pb-2">
                <Shield className="h-8 w-8 text-blue-600 mb-2" />
                <CardTitle className="text-gray-900">Compliance & Security</CardTitle>
              </CardHeader>
              <CardContent>
                <CardDescription className="text-gray-600">
                  Built for regulatory compliance with complete audit trails, FDA data integration, and secure document management.
                </CardDescription>
              </CardContent>
            </Card>

            <Card className="bg-white border-gray-200 shadow-sm hover:shadow-md transition-shadow">
              <CardHeader className="pb-2">
                <TrendingUp className="h-8 w-8 text-blue-600 mb-2" />
                <CardTitle className="text-gray-900">Analytics & Insights</CardTitle>
              </CardHeader>
              <CardContent>
                <CardDescription className="text-gray-600">
                  Advanced reporting, expiry forecasting, and business intelligence for pharmaceutical operations.
                </CardDescription>
              </CardContent>
            </Card>
          </div>
        </div>

        {/* CTA Section */}
        <div className="mt-24 bg-blue-600 rounded-2xl shadow-xl overflow-hidden">
          <div className="px-6 py-12 sm:px-12 sm:py-16 lg:px-16">
            <div className="text-center">
              <h2 className="text-3xl font-bold tracking-tight text-white">
                Ready to streamline your pharmaceutical operations?
              </h2>
              <p className="mt-4 text-lg text-blue-100">
                Start managing your inventory smarter with AI-powered tools.
              </p>
              <div className="mt-8">
                <Link href="/register">
                  <Button size="lg" variant="secondary" className="px-8">
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
      <footer className="bg-white border-t border-gray-200">
        <div className="max-w-7xl mx-auto py-8 px-4 sm:px-6 lg:px-8">
          <div className="flex flex-col sm:flex-row justify-between items-center gap-4">
            <div className="flex items-center space-x-2">
              <AtlasLogo size={24} />
              <span className="text-sm font-medium text-gray-600">
                Atlas PharmaTech
              </span>
            </div>
            <p className="text-sm text-gray-500">
              &copy; 2024 Atlas PharmaTech. Built for the pharmaceutical industry.
            </p>
          </div>
        </div>
      </footer>
    </div>
  )
}
