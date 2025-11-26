'use client';

import { useState } from 'react';
import Link from 'next/link';
import { usePathname, useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  LayoutDashboard,
  Package,
  Store,
  Users,
  Settings,
  LogOut,
  Menu,
  X,
  Search,
  Sparkles,
  Shield,
  Plug,
  ShieldCheck,
  UserCog,
  Activity,
  ShieldAlert,
  Lock
} from 'lucide-react';
import { ProtectedRoute } from './protected-route';
import { AiAssistantSidebar } from './ai-assistant-sidebar';
import { NotificationBell } from './notification-bell';
import { AtlasLogo } from './atlas-logo';

interface DashboardLayoutProps {
  children: React.ReactNode;
}

const navigation = [
  { name: 'Dashboard', href: '/dashboard', icon: LayoutDashboard },
  { name: 'My Inventory', href: '/dashboard/inventory', icon: Package },
  { name: 'AI Import', href: '/dashboard/ai-import', icon: Sparkles },
  { name: 'ERP Integration', href: '/dashboard/erp', icon: Plug },
  { name: 'Regulatory AI', href: '/dashboard/regulatory', icon: Shield },
  { name: 'Marketplace', href: '/dashboard/marketplace', icon: Store },
  { name: 'Pharmaceuticals', href: '/dashboard/pharmaceuticals', icon: Package },
  { name: 'Inquiries', href: '/dashboard/inquiries', icon: Users },
  { name: 'Transactions', href: '/dashboard/transactions', icon: Store },
  { name: 'Settings', href: '/dashboard/settings', icon: Settings },
];

const adminNavigation = [
  { name: 'Admin Dashboard', href: '/dashboard/admin', icon: ShieldCheck },
  { name: 'User Management', href: '/dashboard/admin/users', icon: UserCog },
  { name: 'Verification Queue', href: '/dashboard/admin/verification', icon: ShieldAlert },
  { name: 'Audit Logs', href: '/dashboard/admin/audit-logs', icon: Activity },
  { name: 'Security Monitoring', href: '/dashboard/admin/security', icon: Lock },
];

export function DashboardLayout({ children }: DashboardLayoutProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [aiAssistantOpen, setAiAssistantOpen] = useState(false);
  const [globalSearch, setGlobalSearch] = useState<string>('');
  const pathname = usePathname();
  const router = useRouter();
  const { user, logout, isAdmin } = useAuth();

  const handleLogout = () => {
    logout();
    router.push('/');
  };

  const handleGlobalSearch = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && globalSearch.trim()) {
      // Navigate to marketplace with the search query
      router.push(`/dashboard/marketplace?search=${encodeURIComponent(globalSearch)}`);
      setGlobalSearch(''); // Clear search after navigation
    }
  };

  return (
    <ProtectedRoute>
      <div className="min-h-screen bg-gray-50">
        {/* Mobile sidebar backdrop */}
        {sidebarOpen && (
          <div
            className="fixed inset-0 z-40 lg:hidden bg-gray-600 bg-opacity-75"
            onClick={() => setSidebarOpen(false)}
          />
        )}

        {/* Sidebar */}
        <div className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 bg-white border-r border-gray-200 shadow-lg transform transition-all duration-300 ease-in-out lg:translate-x-0",
          sidebarOpen ? "translate-x-0" : "-translate-x-full"
        )}>
          <div className="flex items-center justify-between h-16 px-6 border-b border-gray-200">
            <Link href="/dashboard" prefetch={true} className="flex items-center gap-2 group">
              <AtlasLogo size={32} className="transition-transform group-hover:scale-110" />
              <span className="text-xl font-bold text-gray-900">Atlas PharmaTech</span>
            </Link>
            <Button
              variant="ghost"
              size="sm"
              className="lg:hidden"
              onClick={() => setSidebarOpen(false)}
            >
              <X className="h-5 w-5" />
            </Button>
          </div>

          <nav className="mt-6 px-3 overflow-y-auto" style={{ maxHeight: 'calc(100vh - 200px)' }}>
            {/* Main Navigation */}
            <div className="space-y-1">
              {navigation.map((item) => {
                const isActive = pathname === item.href;
                return (
                  <Link
                    key={item.name}
                    href={item.href}
                    prefetch={true}
                    className={cn(
                      "flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors",
                      isActive
                        ? "bg-blue-100 text-blue-700"
                        : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                    )}
                    onClick={(e) => {
                      // Don't prevent default - let Link handle navigation
                      setSidebarOpen(false);
                    }}
                  >
                    <item.icon className="mr-3 h-5 w-5" />
                    {item.name}
                  </Link>
                );
              })}
            </div>

            {/* Admin Navigation Section */}
            {isAdmin() && (
              <>
                <div className="my-4 border-t border-gray-200"></div>
                <div className="mb-2 px-3">
                  <p className="text-xs font-semibold text-gray-500 uppercase tracking-wider">
                    Administration
                  </p>
                </div>
                <div className="space-y-1">
                  {adminNavigation.map((item) => {
                    const isActive = pathname === item.href || pathname.startsWith(item.href + '/');
                    return (
                      <Link
                        key={item.name}
                        href={item.href}
                        prefetch={true}
                        className={cn(
                          "flex items-center justify-between px-3 py-2 text-sm font-medium rounded-md transition-colors",
                          isActive
                            ? "bg-purple-100 text-purple-700"
                            : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                        )}
                        onClick={(e) => {
                          // Don't prevent default - let Link handle navigation
                          setSidebarOpen(false);
                        }}
                      >
                        <div className="flex items-center">
                          <item.icon className="mr-3 h-5 w-5" />
                          {item.name}
                        </div>
                        {item.badge && (
                          <Badge variant="destructive" className="ml-auto text-xs">
                            New
                          </Badge>
                        )}
                      </Link>
                    );
                  })}
                </div>
              </>
            )}
          </nav>

          {/* User info and logout */}
          <div className="absolute bottom-0 left-0 right-0 p-4 border-t border-gray-200">
            <div className="flex items-center mb-3">
              <div className="w-8 h-8 bg-blue-600 rounded-full flex items-center justify-center">
                <span className="text-white text-sm font-medium">
                  {user?.company_name.charAt(0).toUpperCase()}
                </span>
              </div>
              <div className="ml-3 flex-1 min-w-0">
                <p className="text-sm font-medium text-gray-900 truncate">
                  {user?.company_name}
                </p>
                <div className="flex items-center gap-1 flex-wrap">
                  {user?.is_verified ? (
                    <Badge variant="secondary" className="text-xs">
                      Verified
                    </Badge>
                  ) : (
                    <Badge variant="outline" className="text-xs">
                      Pending
                    </Badge>
                  )}
                  {user?.role === 'superadmin' && (
                    <Badge className="text-xs bg-purple-100 text-purple-700 border-purple-200">
                      <Shield className="h-2.5 w-2.5 mr-1" />
                      Superadmin
                    </Badge>
                  )}
                  {user?.role === 'admin' && (
                    <Badge variant="default" className="text-xs">
                      <Shield className="h-2.5 w-2.5 mr-1" />
                      Admin
                    </Badge>
                  )}
                </div>
              </div>
            </div>
            <Button
              variant="ghost"
              size="sm"
              className="w-full justify-start text-gray-600 hover:text-gray-900"
              onClick={handleLogout}
            >
              <LogOut className="mr-2 h-4 w-4" />
              Logout
            </Button>
          </div>
        </div>

        {/* Main content */}
        <div className="lg:pl-64">
          {/* Top bar */}
          <div className="sticky top-0 z-30 flex h-16 bg-white shadow-sm border-b border-gray-200">
            <Button
              variant="ghost"
              size="sm"
              className="lg:hidden ml-4"
              onClick={() => setSidebarOpen(true)}
            >
              <Menu className="h-5 w-5" />
            </Button>

            <div className="flex-1 flex items-center justify-between px-4 sm:px-6 lg:px-8">
              <div className="flex items-center flex-1 max-w-2xl">
                <div className="relative w-full">
                  <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-gray-400" />
                  <input
                    type="text"
                    placeholder="Search products, inventory, manufacturers... (Press Enter)"
                    value={globalSearch}
                    onChange={(e) => setGlobalSearch(e.target.value)}
                    onKeyDown={handleGlobalSearch}
                    className="w-full pl-12 pr-4 py-3 bg-gray-50 border border-gray-200 rounded-lg text-sm text-gray-900 placeholder-gray-500 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 focus:bg-white transition-all"
                  />
                </div>
              </div>

              <div className="flex items-center space-x-2">
                <NotificationBell />
              </div>
            </div>
          </div>

          {/* Page content */}
          <main className="flex-1 pt-6">
            {children}
          </main>
        </div>

        {/* Floating AI Assistant Button */}
        <button
          onClick={() => setAiAssistantOpen(true)}
          className="fixed bottom-6 right-6 z-30 p-4 bg-gradient-to-r from-purple-500 to-blue-500 text-white rounded-full shadow-2xl hover:shadow-purple-500/50 hover:scale-110 transition-all duration-200 group"
          aria-label="Open AI Assistant"
        >
          <Sparkles className="h-6 w-6 group-hover:rotate-12 transition-transform" />
          <div className="absolute -top-1 -right-1 w-3 h-3 bg-green-400 rounded-full animate-pulse" />
        </button>

        {/* AI Assistant Sidebar */}
        <AiAssistantSidebar
          isOpen={aiAssistantOpen}
          onClose={() => setAiAssistantOpen(false)}
        />
      </div>
    </ProtectedRoute>
  );
}