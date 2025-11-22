'use client';

import { useEffect, useState, useMemo } from 'react'
import { useRouter } from 'next/navigation'
import { DashboardLayout } from '@/components/dashboard-layout'
import { useAuth } from '@/contexts/auth-context'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Package,
  Store,
  Users,
  TrendingUp,
  AlertTriangle,
  DollarSign,
  Activity,
  BarChart3,
  ArrowUpRight,
  ArrowDownRight,
  Clock,
  ShoppingCart,
  Pill,
  Factory,
  Timer,
  AlertCircle,
  CheckCircle2,
} from 'lucide-react'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { InventoryService, MarketplaceService, PharmaceuticalService } from '@/lib/services'
import { Inventory, Inquiry, Manufacturer } from '@/types/pharmaceutical'
import { toast } from 'react-toastify'

const COLORS = ['#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#14B8A6', '#F97316'];

interface DashboardStats {
  totalInventory: number;
  activeListings: number;
  pendingInquiries: number;
  totalTransactions: number;
  lowStockItems: number;
  expiringItems: number;
  totalValue: number;
  averagePrice: number;
  stockUtilization: number;
  totalRevenue: number;
  monthlyGrowth: number;
  inventoryTurnover: number;
}

export default function DashboardPage() {
  const router = useRouter();
  const { user } = useAuth();
  const [inventory, setInventory] = useState<Inventory[]>([]);
  const [inquiries, setInquiries] = useState<Inquiry[]>([]);
  const [transactions, setTransactions] = useState<any[]>([]);
  const [manufacturers, setManufacturers] = useState<Manufacturer[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const getDaysUntilExpiry = (expiryDate: string) => {
    const today = new Date();
    const expiry = new Date(expiryDate);
    const diffTime = expiry.getTime() - today.getTime();
    return Math.ceil(diffTime / (1000 * 60 * 60 * 24));
  };

  const stats = useMemo(() => {
    if (!inventory.length) {
      return {
        totalInventory: 0,
        activeListings: 0,
        pendingInquiries: 0,
        totalTransactions: transactions.length,
        lowStockItems: 0,
        expiringItems: 0,
        totalValue: 0,
        averagePrice: 0,
        stockUtilization: 0,
        totalRevenue: 0,
        monthlyGrowth: 0,
        inventoryTurnover: 0,
      };
    }

    const activeListings = inventory.filter(item => item.status === 'available').length;
    const lowStockItems = inventory.filter(item => item.quantity < 10).length;
    const pendingInquiries = inquiries.filter(inquiry => inquiry.status === 'pending').length;

    const totalValue = inventory.reduce((sum, item) => {
      return sum + (parseFloat(item.unit_price) * item.quantity);
    }, 0);

    const totalRevenue = transactions.reduce((sum, tx) => {
      return sum + (parseFloat(tx.total_price || '0'));
    }, 0);

    const averagePrice = inventory.length > 0 ? totalValue / inventory.reduce((sum, item) => sum + item.quantity, 0) : 0;
    const stockUtilization = inventory.length > 0 ? (activeListings / inventory.length) * 100 : 0;

    // Calculate monthly growth (comparing current month to last 30 days ago)
    const thirtyDaysAgo = new Date();
    thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);
    const recentTransactions = transactions.filter(tx => new Date(tx.created_at) >= thirtyDaysAgo);
    const recentRevenue = recentTransactions.reduce((sum, tx) => sum + parseFloat(tx.total_price || '0'), 0);
    const monthlyGrowth = totalRevenue > 0 ? ((recentRevenue / totalRevenue) * 100) : 0;

    // Inventory turnover rate (simplified)
    const inventoryTurnover = totalRevenue > 0 && totalValue > 0 ? (totalRevenue / totalValue) * 100 : 0;

    return {
      totalInventory: inventory.length,
      activeListings,
      pendingInquiries,
      totalTransactions: transactions.length,
      lowStockItems,
      expiringItems: inventory.filter(item => {
        const daysUntilExpiry = getDaysUntilExpiry(item.expiry_date);
        return daysUntilExpiry <= 30 && daysUntilExpiry > 0;
      }).length,
      totalValue,
      averagePrice,
      stockUtilization,
      totalRevenue,
      monthlyGrowth,
      inventoryTurnover,
    };
  }, [inventory, inquiries, transactions]);

  const analyticsData = useMemo(() => {
    if (!inventory.length) return {
      categoryDistribution: [],
      statusDistribution: [],
      monthlyTrend: [],
      topProducts: [],
      expiryDistribution: [],
      revenueByCategory: [],
      inquiryStatusBreakdown: [],
    };

    // Auto-detect pharmaceutical category from category field OR drug name
    const normalizeFDACategory = (category: string | undefined, drugName?: string): string => {
      const cat = (category || '').toLowerCase().trim();
      const name = (drugName || '').toLowerCase().trim();

      // Check category field first
      if (cat.includes('antibiotic') || cat.includes('antibacterial')) return 'Antibiotics';
      if (cat.includes('analgesic') || cat.includes('pain') || cat.includes('nsaid')) return 'Analgesics';
      if (cat.includes('cardiovascular') || cat.includes('cardio') || cat.includes('heart')) return 'Cardiovascular';
      if (cat.includes('respiratory') || cat.includes('asthma')) return 'Respiratory';
      if (cat.includes('diabetes') || cat.includes('insulin')) return 'Diabetes';
      if (cat.includes('gastrointestinal') || cat.includes('gastro')) return 'Gastrointestinal';
      if (cat.includes('neurological') || cat.includes('neuro')) return 'Neurological';
      if (cat.includes('oncology') || cat.includes('cancer')) return 'Oncology';
      if (cat.includes('dermatolog') || cat.includes('skin')) return 'Dermatological';
      if (cat.includes('hormone') || cat.includes('endocrine')) return 'Hormonal';
      if (cat.includes('vitamin') || cat.includes('supplement')) return 'Vitamins';
      if (cat.includes('vaccine')) return 'Vaccines';

      // Auto-detect from drug name if category is generic (General/Other/empty)
      if (!cat || cat === 'general' || cat === 'other') {
        // Antibiotics
        if (/amoxicillin|penicillin|cephalexin|azithromycin|ciprofloxacin|doxycycline|metronidazole|clindamycin|erythromycin|ampicillin|tetracycline|vancomycin|gentamicin|levofloxacin|ceftriaxone|augmentin|bactrim|zithromax|cipro|keflex/i.test(name)) return 'Antibiotics';

        // Analgesics/Pain
        if (/aspirin|ibuprofen|acetaminophen|naproxen|tylenol|advil|motrin|aleve|paracetamol|tramadol|morphine|oxycodone|hydrocodone|codeine|fentanyl|percocet|vicodin|celebrex|meloxicam/i.test(name)) return 'Analgesics';

        // Cardiovascular
        if (/lisinopril|metoprolol|amlodipine|atorvastatin|losartan|simvastatin|carvedilol|furosemide|hydrochlorothiazide|warfarin|clopidogrel|digoxin|propranolol|valsartan|nifedipine|diltiazem|lipitor|crestor|plavix|lasix/i.test(name)) return 'Cardiovascular';

        // Gastrointestinal
        if (/omeprazole|pantoprazole|ranitidine|famotidine|esomeprazole|lansoprazole|prilosec|nexium|pepcid|zantac|protonix|prevacid|metoclopramide|ondansetron|loperamide|bismuth|antacid|laxative/i.test(name)) return 'Gastrointestinal';

        // Diabetes
        if (/metformin|insulin|glipizide|glyburide|sitagliptin|empagliflozin|liraglutide|glucophage|januvia|jardiance|lantus|humalog|novolog|ozempic|trulicity/i.test(name)) return 'Diabetes';

        // Respiratory
        if (/albuterol|fluticasone|montelukast|budesonide|ipratropium|salmeterol|tiotropium|ventolin|proair|advair|symbicort|singulair|prednisone|theophylline/i.test(name)) return 'Respiratory';

        // Neurological/Psychiatric
        if (/sertraline|fluoxetine|escitalopram|duloxetine|venlafaxine|bupropion|alprazolam|lorazepam|diazepam|clonazepam|gabapentin|pregabalin|lamotrigine|carbamazepine|phenytoin|levetiracetam|zoloft|prozac|lexapro|cymbalta|xanax|ativan|valium|neurontin|lyrica/i.test(name)) return 'Neurological';

        // Dermatological
        if (/hydrocortisone|betamethasone|triamcinolone|clobetasol|ketoconazole|clotrimazole|mupirocin|tretinoin|adapalene|benzoyl|permethrin|ivermectin/i.test(name)) return 'Dermatological';

        // Hormonal
        if (/levothyroxine|prednisone|methylprednisolone|estradiol|progesterone|testosterone|synthroid|armour thyroid/i.test(name)) return 'Hormonal';

        // Vitamins/Supplements
        if (/vitamin|calcium|iron|zinc|magnesium|folic acid|b12|d3|omega|multivitamin|supplement/i.test(name)) return 'Vitamins';
      }

      // Return original category if specific, otherwise Other
      if (category && cat !== 'general') {
        return category.charAt(0).toUpperCase() + category.slice(1);
      }
      return 'Other';
    };

    // Category distribution - auto-detects category from drug name if needed
    const categoryMap = new Map<string, { count: number; quantity: number; value: number }>();
    inventory.forEach((item) => {
      const drugName = item.pharmaceutical?.brand_name || item.pharmaceutical?.generic_name || '';
      const category = normalizeFDACategory(item.pharmaceutical?.category, drugName);
      const current = categoryMap.get(category) || { count: 0, quantity: 0, value: 0 };
      categoryMap.set(category, {
        count: current.count + 1,
        quantity: current.quantity + item.quantity,
        value: current.value + (parseFloat(item.unit_price) * item.quantity),
      });
    });

    const totalValue = Array.from(categoryMap.values()).reduce((sum, data) => sum + data.value, 0);
    const categoryDistribution = Array.from(categoryMap.entries())
      .map(([name, data]) => ({
        name,
        value: data.value,
        count: data.count,
        quantity: data.quantity,
        percent: totalValue > 0 ? (data.value / totalValue) * 100 : 0,
      }))
      .sort((a, b) => b.value - a.value)
      .slice(0, 8); // Top 8 categories

    // Status distribution with colors
    const statusMap = new Map<string, number>();
    inventory.forEach((item) => {
      statusMap.set(item.status, (statusMap.get(item.status) || 0) + 1);
    });
    const statusDistribution = Array.from(statusMap.entries()).map(([name, value]) => ({
      name: name.charAt(0).toUpperCase() + name.slice(1),
      value,
      fill: name === 'available' ? '#10B981' : name === 'reserved' ? '#F59E0B' : '#EF4444'
    }));

    // Monthly trend (last 6 months)
    const monthlyTrend = Array.from({ length: 6 }, (_, i) => {
      const date = new Date();
      date.setMonth(date.getMonth() - (5 - i));
      const monthName = date.toLocaleString('default', { month: 'short' });

      const monthItems = inventory.filter(item => {
        const itemDate = new Date(item.created_at);
        return itemDate.getMonth() === date.getMonth() &&
               itemDate.getFullYear() === date.getFullYear();
      });

      const monthTransactions = transactions.filter(tx => {
        const txDate = new Date(tx.created_at);
        return txDate.getMonth() === date.getMonth() &&
               txDate.getFullYear() === date.getFullYear();
      });

      return {
        month: monthName,
        items: monthItems.length,
        value: monthItems.reduce((sum, item) => sum + (parseFloat(item.unit_price) * item.quantity), 0),
        revenue: monthTransactions.reduce((sum, tx) => sum + parseFloat(tx.total_price || '0'), 0),
        transactions: monthTransactions.length,
      };
    });

    // Top products by value
    const topProducts = inventory
      .map(item => {
        const drugName = item.pharmaceutical?.brand_name || item.pharmaceutical?.generic_name || '';
        return {
          name: item.pharmaceutical?.brand_name || 'Unknown',
          value: parseFloat(item.unit_price) * item.quantity,
          quantity: item.quantity,
          category: normalizeFDACategory(item.pharmaceutical?.category, drugName),
        };
      })
      .sort((a, b) => b.value - a.value)
      .slice(0, 5);

    // Expiry distribution
    const expiryRanges = [
      { name: 'Expired', min: -Infinity, max: 0, color: '#EF4444' },
      { name: '0-30 days', min: 0, max: 30, color: '#F59E0B' },
      { name: '31-90 days', min: 30, max: 90, color: '#FBBF24' },
      { name: '3-6 months', min: 90, max: 180, color: '#10B981' },
      { name: '6+ months', min: 180, max: Infinity, color: '#3B82F6' },
    ];

    const expiryDistribution = expiryRanges.map(range => ({
      name: range.name,
      value: inventory.filter(item => {
        const days = getDaysUntilExpiry(item.expiry_date);
        return days > range.min && days <= range.max;
      }).length,
      fill: range.color,
    }));

    // Inquiry status breakdown
    const inquiryStatusMap = new Map<string, number>();
    inquiries.forEach((inquiry) => {
      inquiryStatusMap.set(inquiry.status, (inquiryStatusMap.get(inquiry.status) || 0) + 1);
    });
    const inquiryStatusBreakdown = Array.from(inquiryStatusMap.entries()).map(([name, value]) => ({
      name: name.charAt(0).toUpperCase() + name.slice(1),
      value
    }));

    return {
      categoryDistribution,
      statusDistribution,
      monthlyTrend,
      topProducts,
      expiryDistribution,
      inquiryStatusBreakdown,
    };
  }, [inventory, inquiries, transactions]);

  // Critical alerts and insights
  const insights = useMemo(() => {
    const criticalAlerts = [];
    const recommendations = [];

    if (stats.expiringItems > 0) {
      criticalAlerts.push({
        type: 'warning',
        icon: Timer,
        title: 'Items Expiring Soon',
        message: `${stats.expiringItems} item${stats.expiringItems > 1 ? 's' : ''} expiring within 30 days`,
        action: 'View Details',
        link: '/dashboard/inventory?filter=expiring',
        onClick: null
      });
    }

    if (stats.lowStockItems > 0) {
      // Get the first low stock item to search for in marketplace
      const lowStockItem = inventory.find(item => item.quantity < 10);
      const searchQuery = lowStockItem?.pharmaceutical?.brand_name ||
                         lowStockItem?.pharmaceutical?.generic_name || '';

      criticalAlerts.push({
        type: 'error',
        icon: AlertCircle,
        title: 'Low Stock Alert',
        message: `${stats.lowStockItems} item${stats.lowStockItems > 1 ? 's' : ''} below minimum threshold`,
        action: 'Find Suppliers',
        link: `/dashboard/marketplace${searchQuery ? `?search=${encodeURIComponent(searchQuery)}` : ''}`,
        onClick: null
      });
    }

    if (stats.pendingInquiries > 5) {
      criticalAlerts.push({
        type: 'info',
        icon: Users,
        title: 'Pending Inquiries',
        message: `${stats.pendingInquiries} inquiries awaiting response`,
        action: 'Review Inquiries',
        link: '/dashboard/inquiries'
      });
    }

    // Recommendations
    if (stats.stockUtilization < 50) {
      recommendations.push({
        title: 'Improve Stock Utilization',
        message: `Only ${stats.stockUtilization.toFixed(0)}% of inventory is actively listed. Consider listing more items.`,
        icon: TrendingUp,
      });
    }

    if (analyticsData.topProducts.length > 0) {
      const topProduct = analyticsData.topProducts[0];
      recommendations.push({
        title: 'Top Performer',
        message: `${topProduct.name} is your highest value item at $${topProduct.value.toLocaleString()}`,
        icon: TrendingUp,
      });
    }

    if (stats.monthlyGrowth > 10) {
      recommendations.push({
        title: 'Strong Growth',
        message: `Revenue increased by ${stats.monthlyGrowth.toFixed(1)}% in the last 30 days`,
        icon: ArrowUpRight,
      });
    }

    return { criticalAlerts, recommendations };
  }, [stats, analyticsData]);

  useEffect(() => {
    loadDashboardData();
  }, []);

  const loadDashboardData = async () => {
    try {
      setIsLoading(true);

      const [inventoryData, inquiriesData, transactionsData, manufacturersData] = await Promise.all([
        InventoryService.getUserInventory(),
        MarketplaceService.getBuyerInquiries(),
        MarketplaceService.getUserTransactions(),
        PharmaceuticalService.getManufacturers(),
      ]);

      setInventory(inventoryData);
      setInquiries(inquiriesData);
      setTransactions(transactionsData);
      setManufacturers(manufacturersData);
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
      toast.error('Failed to load dashboard data');
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return (
      <DashboardLayout>
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center space-y-4">
            <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-blue-600 mx-auto"></div>
            <p className="text-gray-600 font-medium">Loading your dashboard...</p>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="p-6 space-y-6">
        {/* Header with personalized greeting */}
        <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 shadow-sm">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-3">
                <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
                  {user?.company_name}
                </h1>
                {user?.is_verified && (
                  <Badge variant="default" className="bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-400 border-green-200 dark:border-green-700">
                    Verified
                  </Badge>
                )}
              </div>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Dashboard Overview • Last updated {new Date().toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
              </p>
            </div>
            <div className="text-right">
              <div className="text-sm text-gray-500 dark:text-gray-400">Current Period</div>
              <div className="text-lg font-semibold text-gray-900 dark:text-white">
                {new Date().toLocaleDateString('en-US', { month: 'long', year: 'numeric' })}
              </div>
            </div>
          </div>
        </div>

        {/* Critical Alerts Section */}
        {insights.criticalAlerts.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {insights.criticalAlerts.map((alert, idx) => (
              <Card key={idx} className={`border-l-4 ${
                alert.type === 'error' ? 'border-l-red-500 bg-red-50 dark:bg-red-900/20' :
                alert.type === 'warning' ? 'border-l-orange-500 bg-orange-50 dark:bg-orange-900/20' :
                'border-l-blue-500 bg-blue-50 dark:bg-blue-900/20'
              }`}>
                <CardContent className="pt-6">
                  <div className="flex items-start gap-4">
                    <div className={`p-3 rounded-lg ${
                      alert.type === 'error' ? 'bg-red-100 dark:bg-red-900/40' :
                      alert.type === 'warning' ? 'bg-orange-100 dark:bg-orange-900/40' :
                      'bg-blue-100 dark:bg-blue-900/40'
                    }`}>
                      <alert.icon className={`h-6 w-6 ${
                        alert.type === 'error' ? 'text-red-600 dark:text-red-400' :
                        alert.type === 'warning' ? 'text-orange-600 dark:text-orange-400' :
                        'text-blue-600 dark:text-blue-400'
                      }`} />
                    </div>
                    <div className="flex-1">
                      <h3 className="font-semibold text-gray-900 dark:text-white mb-1">{alert.title}</h3>
                      <p className="text-sm text-gray-600 dark:text-gray-300 mb-3">{alert.message}</p>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => router.push(alert.link)}
                        className="text-xs"
                      >
                        {alert.action}
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}

        {/* Enhanced Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {/* Total Inventory Value */}
          <Card className="border-l-4 border-l-blue-600 shadow-sm hover:shadow-md transition-shadow">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-gray-600">Inventory Value</CardTitle>
              <div className="h-10 w-10 rounded-lg bg-blue-50 flex items-center justify-center">
                <DollarSign className="h-5 w-5 text-blue-600" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900">${stats.totalValue.toLocaleString()}</div>
              <p className="text-xs text-gray-500 mt-2 flex items-center gap-1">
                <TrendingUp className="h-3 w-3" />
                {stats.totalInventory} total items
              </p>
              <div className="mt-3 pt-3 border-t border-gray-100">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Avg Price</span>
                  <span className="font-semibold text-gray-900">${stats.averagePrice.toFixed(2)}</span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Revenue */}
          <Card className="border-l-4 border-l-green-600 shadow-sm hover:shadow-md transition-shadow">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-gray-600">Total Revenue</CardTitle>
              <div className="h-10 w-10 rounded-lg bg-green-50 flex items-center justify-center">
                <ShoppingCart className="h-5 w-5 text-green-600" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900">${stats.totalRevenue.toLocaleString()}</div>
              <p className="text-xs mt-2 flex items-center gap-1">
                {stats.monthlyGrowth > 0 ? (
                  <>
                    <ArrowUpRight className="h-3 w-3 text-green-600" />
                    <span className="text-green-600 font-medium">+{stats.monthlyGrowth.toFixed(1)}%</span>
                  </>
                ) : (
                  <>
                    <ArrowDownRight className="h-3 w-3 text-red-600" />
                    <span className="text-red-600 font-medium">{stats.monthlyGrowth.toFixed(1)}%</span>
                  </>
                )}
                <span className="text-gray-500 ml-1">vs last month</span>
              </p>
              <div className="mt-3 pt-3 border-t border-gray-100">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Transactions</span>
                  <span className="font-semibold text-gray-900">{stats.totalTransactions}</span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Active Listings */}
          <Card className="border-l-4 border-l-purple-600 shadow-sm hover:shadow-md transition-shadow">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-gray-600">Active Listings</CardTitle>
              <div className="h-10 w-10 rounded-lg bg-purple-50 flex items-center justify-center">
                <Store className="h-5 w-5 text-purple-600" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900">{stats.activeListings}</div>
              <p className="text-xs text-gray-500 mt-2 flex items-center gap-1">
                <Activity className="h-3 w-3" />
                {stats.stockUtilization.toFixed(1)}% utilization
              </p>
              <div className="mt-3 pt-3 border-t border-gray-100">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Pending</span>
                  <span className="font-semibold text-gray-900">{stats.pendingInquiries} inquiries</span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Health Score */}
          <Card className={`border-l-4 ${
            stats.lowStockItems === 0 && stats.expiringItems === 0 ? 'border-l-green-600' :
            stats.lowStockItems + stats.expiringItems < 5 ? 'border-l-yellow-600' : 'border-l-red-600'
          } shadow-sm hover:shadow-md transition-shadow`}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-gray-600">Health Score</CardTitle>
              <div className={`h-10 w-10 rounded-lg flex items-center justify-center ${
                stats.lowStockItems === 0 && stats.expiringItems === 0 ? 'bg-green-50' :
                stats.lowStockItems + stats.expiringItems < 5 ? 'bg-yellow-50' : 'bg-red-50'
              }`}>
                <Activity className={`h-5 w-5 ${
                  stats.lowStockItems === 0 && stats.expiringItems === 0 ? 'text-green-600' :
                  stats.lowStockItems + stats.expiringItems < 5 ? 'text-yellow-600' : 'text-red-600'
                }`} />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-gray-900">
                {stats.lowStockItems === 0 && stats.expiringItems === 0 ? 'Excellent' :
                 stats.lowStockItems + stats.expiringItems < 5 ? 'Good' : 'Action Needed'}
              </div>
              <p className="text-xs text-gray-500 mt-2 flex items-center gap-1">
                <AlertTriangle className="h-3 w-3" />
                {stats.lowStockItems + stats.expiringItems} total alerts
              </p>
              <div className="mt-3 pt-3 border-t border-gray-100">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Low Stock</span>
                  <span className="font-semibold text-gray-900">{stats.lowStockItems}</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Main Analytics Section */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Category Distribution - 2 columns */}
          <Card className="lg:col-span-2 shadow-md">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Pill className="h-5 w-5 text-blue-600" />
                    Inventory by Category
                  </CardTitle>
                  <p className="text-sm text-gray-600 mt-1">Distribution of pharmaceutical categories by value</p>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={320}>
                <PieChart>
                  <Pie
                    data={analyticsData.categoryDistribution}
                    cx="40%"
                    cy="50%"
                    labelLine={false}
                    label={false}
                    outerRadius={110}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {analyticsData.categoryDistribution.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value, name, props) => [
                      `$${Number(value).toLocaleString()}`,
                      `${props.payload.count} items (${props.payload.percent.toFixed(1)}%)`
                    ]}
                    contentStyle={{ fontSize: '12px', borderRadius: '8px', border: '1px solid #e5e7eb' }}
                  />
                  <Legend
                    layout="vertical"
                    align="right"
                    verticalAlign="middle"
                    wrapperStyle={{ fontSize: '12px', paddingLeft: '20px' }}
                    formatter={(value, entry: any) => {
                      const percent = entry?.payload?.percent || 0;
                      return `${value} (${percent.toFixed(0)}%)`;
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Quick Stats - 1 column */}
          <Card className="shadow-md">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="h-5 w-5 text-purple-600" />
                Quick Insights
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Stock Status */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <span className="text-sm font-medium text-gray-700">Stock Status</span>
                  <Badge variant={stats.lowStockItems > 5 ? "destructive" : "default"}>
                    {stats.lowStockItems > 5 ? "Action Needed" : "Healthy"}
                  </Badge>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-green-500 h-2 rounded-full transition-all"
                    style={{ width: `${Math.max(0, 100 - (stats.lowStockItems / stats.totalInventory * 100))}%` }}
                  ></div>
                </div>
              </div>

              {/* Utilization */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <span className="text-sm font-medium text-gray-700">Utilization Rate</span>
                  <span className="text-sm font-semibold text-gray-900">{stats.stockUtilization.toFixed(0)}%</span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-blue-500 h-2 rounded-full transition-all"
                    style={{ width: `${stats.stockUtilization}%` }}
                  ></div>
                </div>
              </div>

              {/* Key Metrics List */}
              <div className="space-y-3 pt-2 border-t">
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-600 flex items-center gap-2">
                    <Factory className="h-4 w-4" />
                    Manufacturers
                  </span>
                  <span className="font-semibold text-gray-900">{manufacturers.length}</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-600 flex items-center gap-2">
                    <Package className="h-4 w-4" />
                    Unique Products
                  </span>
                  <span className="font-semibold text-gray-900">{stats.totalInventory}</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-600 flex items-center gap-2">
                    <Timer className="h-4 w-4" />
                    Expiring Soon
                  </span>
                  <Badge variant={stats.expiringItems > 0 ? "destructive" : "secondary"}>
                    {stats.expiringItems}
                  </Badge>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-600 flex items-center gap-2">
                    <Users className="h-4 w-4" />
                    Avg Response
                  </span>
                  <span className="font-semibold text-gray-900">24h</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Revenue & Performance Trends */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Monthly Trend */}
          <Card className="shadow-md">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <TrendingUp className="h-5 w-5 text-green-600" />
                    Revenue & Inventory Trend
                  </CardTitle>
                  <p className="text-sm text-gray-600 mt-1">Last 6 months performance</p>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={280}>
                <AreaChart data={analyticsData.monthlyTrend}>
                  <defs>
                    <linearGradient id="colorRevenue" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#10B981" stopOpacity={0.8}/>
                      <stop offset="95%" stopColor="#10B981" stopOpacity={0}/>
                    </linearGradient>
                    <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#3B82F6" stopOpacity={0.8}/>
                      <stop offset="95%" stopColor="#3B82F6" stopOpacity={0}/>
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                  <XAxis dataKey="month" stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <YAxis stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <Tooltip
                    contentStyle={{ borderRadius: '8px', border: '1px solid #e5e7eb' }}
                    formatter={(value) => [`$${Number(value).toLocaleString()}`, '']}
                  />
                  <Legend />
                  <Area
                    type="monotone"
                    dataKey="revenue"
                    stroke="#10B981"
                    fillOpacity={1}
                    fill="url(#colorRevenue)"
                    name="Revenue"
                  />
                  <Area
                    type="monotone"
                    dataKey="value"
                    stroke="#3B82F6"
                    fillOpacity={1}
                    fill="url(#colorValue)"
                    name="Inventory Value"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Top Products */}
          <Card className="shadow-md">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <TrendingUp className="h-5 w-5 text-orange-600" />
                    Top Products by Value
                  </CardTitle>
                  <p className="text-sm text-gray-600 mt-1">Your highest value inventory items</p>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {analyticsData.topProducts.map((product, idx) => (
                  <div key={idx} className="flex items-center gap-4">
                    <div className="flex-shrink-0 w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-500 rounded-lg flex items-center justify-center text-white font-bold">
                      {idx + 1}
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-gray-900 truncate">{product.name}</p>
                      <p className="text-xs text-gray-500">{product.category} • {product.quantity} units</p>
                    </div>
                    <div className="text-right">
                      <p className="font-semibold text-gray-900">${product.value.toLocaleString()}</p>
                    </div>
                  </div>
                ))}
                {analyticsData.topProducts.length === 0 && (
                  <p className="text-center text-gray-500 py-8">No products available</p>
                )}
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Status & Expiry Analysis */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Status Distribution */}
          <Card className="shadow-md">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Activity className="h-5 w-5 text-blue-600" />
                Inventory Status
              </CardTitle>
              <p className="text-sm text-gray-600">Current status of all inventory items</p>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={260}>
                <BarChart data={analyticsData.statusDistribution}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                  <XAxis dataKey="name" stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <YAxis stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <Tooltip contentStyle={{ borderRadius: '8px', border: '1px solid #e5e7eb' }} />
                  <Bar dataKey="value" radius={[8, 8, 0, 0]}>
                    {analyticsData.statusDistribution.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.fill} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          {/* Expiry Distribution */}
          <Card className="shadow-md">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Clock className="h-5 w-5 text-orange-600" />
                Expiry Timeline
              </CardTitle>
              <p className="text-sm text-gray-600">Items grouped by expiration timeframe</p>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={260}>
                <BarChart data={analyticsData.expiryDistribution}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                  <XAxis dataKey="name" stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <YAxis stroke="#6B7280" style={{ fontSize: '12px' }} />
                  <Tooltip contentStyle={{ borderRadius: '8px', border: '1px solid #e5e7eb' }} />
                  <Bar dataKey="value" radius={[8, 8, 0, 0]}>
                    {analyticsData.expiryDistribution.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.fill} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </div>

        {/* Personalized Recommendations */}
        {insights.recommendations.length > 0 && (
          <Card className="shadow-md border-l-4 border-l-blue-500">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <CheckCircle2 className="h-5 w-5 text-blue-600" />
                Personalized Insights
              </CardTitle>
              <p className="text-sm text-gray-600">AI-powered recommendations for your business</p>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {insights.recommendations.map((rec, idx) => (
                  <div key={idx} className="flex items-start gap-3 p-4 bg-blue-50 rounded-lg">
                    <div className="p-2 bg-blue-100 rounded-lg">
                      <rec.icon className="h-5 w-5 text-blue-600" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-semibold text-gray-900 mb-1">{rec.title}</h4>
                      <p className="text-sm text-gray-600">{rec.message}</p>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </DashboardLayout>
  );
}
