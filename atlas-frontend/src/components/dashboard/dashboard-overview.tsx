'use client'

import { useEffect, useState } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useAuthStore } from '@/lib/auth-store'
import { inventoryApi, marketplaceApi } from '@/lib/api'
import { Inventory, Inquiry, Transaction } from '@/types/pharmaceutical'
import {
  Package,
  AlertTriangle,
  TrendingUp,
  ShoppingCart,
  Plus,
  Eye,
} from 'lucide-react'
import Link from 'next/link'

interface DashboardStats {
  totalInventory: number
  expiringSoon: number
  activeInquiries: number
  recentTransactions: number
}

export function DashboardOverview() {
  const { user } = useAuthStore()
  const [stats, setStats] = useState<DashboardStats>({
    totalInventory: 0,
    expiringSoon: 0,
    activeInquiries: 0,
    recentTransactions: 0,
  })
  const [recentItems, setRecentItems] = useState<{
    inventory: Inventory[]
    inquiries: Inquiry[]
    transactions: Transaction[]
  }>({
    inventory: [],
    inquiries: [],
    transactions: [],
  })
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const fetchDashboardData = async () => {
      try {
        // Fetch dashboard stats
        const [inventoryResponse, inquiriesResponse, transactionsResponse, expiringResponse] = await Promise.all([
          inventoryApi.getMyInventory({ limit: 1 }).catch(() => ({ data: { data: [], pagination: { total: 0 } } })),
          marketplaceApi.getInquiries({ limit: 1 }).catch(() => ({ data: { data: [], pagination: { total: 0 } } })),
          marketplaceApi.getTransactions({ limit: 5 }).catch(() => ({ data: { data: [], pagination: { total: 0 } } })),
          inventoryApi.getExpiring(30).catch(() => []),
        ])

        setStats({
          totalInventory: (inventoryResponse as any)?.data?.pagination?.total || 0,
          expiringSoon: expiringResponse.length || 0,
          activeInquiries: (inquiriesResponse as any)?.data?.pagination?.total || 0,
          recentTransactions: (transactionsResponse as any)?.data?.pagination?.total || 0,
        })

        // Fetch recent items
        const [recentInventory, recentInquiries, recentTransactions] = await Promise.all([
          inventoryApi.getMyInventory({ limit: 5 }).catch(() => ({ data: { data: [] } })),
          marketplaceApi.getInquiries({ limit: 5 }).catch(() => ({ data: { data: [] } })),
          marketplaceApi.getTransactions({ limit: 5 }).catch(() => ({ data: { data: [] } })),
        ])

        setRecentItems({
          inventory: recentInventory.data?.data || [],
          inquiries: recentInquiries.data?.data || [],
          transactions: recentTransactions.data?.data || [],
        })
      } catch (error) {
        console.error('Error fetching dashboard data:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchDashboardData()
  }, [])

  const formatExpiryDate = (date: string) => {
    const expiry = new Date(date)
    const now = new Date()
    const diffTime = expiry.getTime() - now.getTime()
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24))

    if (diffDays < 0) return 'Expired'
    if (diffDays === 0) return 'Expires today'
    if (diffDays === 1) return 'Expires tomorrow'
    if (diffDays <= 7) return `Expires in ${diffDays} days`
    if (diffDays <= 30) return `Expires in ${diffDays} days`
    return expiry.toLocaleDateString()
  }

  const getExpiryStatus = (date: string) => {
    const expiry = new Date(date)
    const now = new Date()
    const diffTime = expiry.getTime() - now.getTime()
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24))

    if (diffDays < 0) return 'destructive'
    if (diffDays <= 30) return 'warning'
    return 'default'
  }

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {[...Array(4)].map((_, i) => (
            <Card key={i} className="animate-pulse">
              <CardHeader className="space-y-0 pb-2">
                <div className="h-4 bg-gray-200 rounded w-3/4"></div>
              </CardHeader>
              <CardContent>
                <div className="h-8 bg-gray-200 rounded w-1/2"></div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Welcome Section */}
      <div className="bg-white rounded-lg shadow p-6">
        <h1 className="text-2xl font-bold text-gray-900">
          Welcome back, {user?.contact_person}!
        </h1>
        <p className="text-gray-600 mt-1">
          Here's what's happening with your {user?.company_type} inventory today.
        </p>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Inventory</CardTitle>
            <Package className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.totalInventory}</div>
            <p className="text-xs text-muted-foreground">
              Active pharmaceutical items
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Expiring Soon</CardTitle>
            <AlertTriangle className="h-4 w-4 text-orange-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-orange-600">{stats.expiringSoon}</div>
            <p className="text-xs text-muted-foreground">
              Items expiring within 30 days
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Inquiries</CardTitle>
            <ShoppingCart className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.activeInquiries}</div>
            <p className="text-xs text-muted-foreground">
              Pending buyer inquiries
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Transactions</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.recentTransactions}</div>
            <p className="text-xs text-muted-foreground">
              Total transactions
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
          <CardDescription>
            Common tasks to manage your pharmaceutical inventory
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Link href="/dashboard/inventory/new">
              <Button className="w-full justify-start">
                <Plus className="mr-2 h-4 w-4" />
                Add Inventory Item
              </Button>
            </Link>
            <Link href="/dashboard/marketplace">
              <Button variant="outline" className="w-full justify-start">
                <Search className="mr-2 h-4 w-4" />
                Browse Marketplace
              </Button>
            </Link>
            <Link href="/dashboard/inventory/expiring">
              <Button variant="outline" className="w-full justify-start">
                <AlertTriangle className="mr-2 h-4 w-4" />
                Check Expiring Items
              </Button>
            </Link>
          </div>
        </CardContent>
      </Card>

      {/* Recent Items */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Recent Inventory */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Recent Inventory</CardTitle>
              <Link href="/dashboard/inventory">
                <Button variant="ghost" size="sm">
                  <Eye className="h-4 w-4" />
                </Button>
              </Link>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentItems.inventory.slice(0, 3).map((item) => (
                <div key={item.id} className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">
                      {item.pharmaceutical?.product_name}
                    </p>
                    <p className="text-sm text-gray-500">
                      Qty: {item.quantity} | Batch: {item.batch_number}
                    </p>
                  </div>
                  <Badge variant={getExpiryStatus(item.expiry_date)}>
                    {formatExpiryDate(item.expiry_date)}
                  </Badge>
                </div>
              ))}
              {recentItems.inventory.length === 0 && (
                <p className="text-sm text-gray-500 text-center py-4">
                  No inventory items yet
                </p>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Recent Inquiries */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Recent Inquiries</CardTitle>
              <Link href="/dashboard/inquiries">
                <Button variant="ghost" size="sm">
                  <Eye className="h-4 w-4" />
                </Button>
              </Link>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentItems.inquiries.slice(0, 3).map((inquiry) => (
                <div key={inquiry.id} className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">
                      {inquiry.pharmaceutical?.product_name}
                    </p>
                    <p className="text-sm text-gray-500">
                      Qty: {inquiry.quantity_requested}
                    </p>
                  </div>
                  <Badge variant={inquiry.status === 'pending' ? 'warning' : 'default'}>
                    {inquiry.status}
                  </Badge>
                </div>
              ))}
              {recentItems.inquiries.length === 0 && (
                <p className="text-sm text-gray-500 text-center py-4">
                  No inquiries yet
                </p>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Recent Transactions */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Recent Transactions</CardTitle>
              <Link href="/dashboard/transactions">
                <Button variant="ghost" size="sm">
                  <Eye className="h-4 w-4" />
                </Button>
              </Link>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentItems.transactions.slice(0, 3).map((transaction) => (
                <div key={transaction.id} className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">
                      {transaction.pharmaceutical?.product_name}
                    </p>
                    <p className="text-sm text-gray-500">
                      Qty: {transaction.quantity}
                    </p>
                  </div>
                  <Badge variant={transaction.status === 'completed' ? 'default' : 'warning'}>
                    {transaction.status}
                  </Badge>
                </div>
              ))}
              {recentItems.transactions.length === 0 && (
                <p className="text-sm text-gray-500 text-center py-4">
                  No transactions yet
                </p>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}