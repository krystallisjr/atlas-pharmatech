'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { Shield, AlertTriangle } from 'lucide-react';

interface AdminLayoutProps {
  children: React.ReactNode;
}

export default function AdminLayout({ children }: AdminLayoutProps) {
  const { user, isLoading, isAdmin } = useAuth();
  const router = useRouter();

  useEffect(() => {
    // Redirect non-admin users to dashboard
    if (!isLoading && (!user || !isAdmin())) {
      console.warn('🚫 Admin access denied - redirecting to dashboard');
      router.push('/dashboard');
    }
  }, [user, isLoading, isAdmin, router]);

  // Show loading state
  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
        <div className="text-center space-y-4">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p className="text-gray-600 dark:text-gray-400">Loading admin dashboard...</p>
        </div>
      </div>
    );
  }

  // Show unauthorized message if not admin (brief flash before redirect)
  if (!user || !isAdmin()) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
        <div className="max-w-md p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg text-center space-y-4">
          <AlertTriangle className="h-16 w-16 text-red-500 mx-auto" />
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Access Denied
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            You do not have permission to access the admin dashboard.
          </p>
          <p className="text-sm text-gray-500 dark:text-gray-500">
            Redirecting to dashboard...
          </p>
        </div>
      </div>
    );
  }

  // Render admin content
  return (
    <div className="admin-dashboard">
      {/* Admin header indicator */}
      <div className="bg-blue-600 text-white px-4 py-2 flex items-center justify-center gap-2 shadow-md">
        <Shield className="h-4 w-4" />
        <span className="text-sm font-medium">
          Admin Dashboard
          {user.role === 'superadmin' && ' (Superadmin)'}
        </span>
      </div>

      {/* Admin content */}
      <div className="p-0">
        {children}
      </div>
    </div>
  );
}
