'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { AuthService } from '@/lib/services/auth-service';
import type { User } from '@/types/auth';
import { toast } from 'react-toastify';
import {
  User as UserIcon,
  Building2,
  Phone,
  MapPin,
  FileText,
  Bell,
  Shield,
  Loader2,
  Save,
  ArrowRight,
  LogOut,
  Trash2,
} from 'lucide-react';
import Link from 'next/link';

export default function SettingsPage() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [formData, setFormData] = useState({
    company_name: '',
    contact_person: '',
    phone: '',
    address: '',
    license_number: '',
  });

  useEffect(() => {
    loadProfile();
  }, []);

  const loadProfile = async () => {
    try {
      setIsLoading(true);
      const profileData = await AuthService.getProfile();
      setUser(profileData);
      setFormData({
        company_name: profileData.company_name || '',
        contact_person: profileData.contact_person || '',
        phone: profileData.phone || '',
        address: profileData.address || '',
        license_number: profileData.license_number || '',
      });
    } catch (error) {
      console.error('Failed to load profile:', error);
      toast.error('Failed to load profile');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setIsSaving(true);
      const updated = await AuthService.updateProfile(formData);
      setUser(updated);

      // Update stored user data
      const { token } = AuthService.getStoredAuthData();
      if (token) {
        AuthService.storeAuthData({ user: updated, token, expires_in: 3600 });
      }

      toast.success('Profile updated successfully');
    } catch (error) {
      console.error('Failed to update profile:', error);
      toast.error('Failed to update profile');
    } finally {
      setIsSaving(false);
    }
  };

  const handleLogout = () => {
    AuthService.clearAuthData();
    router.push('/login');
    toast.info('Logged out successfully');
  };

  const handleDeleteAccount = async () => {
    const confirmed = confirm(
      'Are you sure you want to delete your account? This action cannot be undone and will permanently delete all your data including inventory, orders, and transaction history.'
    );

    if (!confirmed) return;

    const doubleConfirmed = confirm(
      'FINAL WARNING: This will permanently delete your account and ALL associated data. Type your company name to confirm.'
    );

    if (!doubleConfirmed) return;

    try {
      await AuthService.deleteAccount();
      AuthService.clearAuthData();
      router.push('/login');
      toast.success('Account deleted successfully');
    } catch (error) {
      console.error('Failed to delete account:', error);
      toast.error('Failed to delete account');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
      </div>
    );
  }

  if (!user) {
    return (
      <div className="text-center text-gray-500 py-12">
        Failed to load profile
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Settings</h1>
        <p className="text-gray-600 mt-2">
          Manage your account settings and preferences
        </p>
      </div>

      {/* Profile Information */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <UserIcon className="h-5 w-5" />
            Profile Information
          </CardTitle>
          <CardDescription>
            Update your company and contact information
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Email (Read-only) */}
          <div className="space-y-2">
            <Label htmlFor="email">Email Address</Label>
            <Input
              id="email"
              type="email"
              value={user.email}
              disabled
              className="bg-gray-50"
            />
            <p className="text-xs text-gray-500">
              Email cannot be changed. Contact support if you need to update your email.
            </p>
          </div>

          {/* Company Name */}
          <div className="space-y-2">
            <Label htmlFor="company_name">
              <div className="flex items-center gap-2">
                <Building2 className="h-4 w-4 text-gray-500" />
                Company Name *
              </div>
            </Label>
            <Input
              id="company_name"
              value={formData.company_name}
              onChange={(e) => setFormData({ ...formData, company_name: e.target.value })}
              placeholder="Your Company Inc."
              required
            />
          </div>

          {/* Contact Person */}
          <div className="space-y-2">
            <Label htmlFor="contact_person">
              <div className="flex items-center gap-2">
                <UserIcon className="h-4 w-4 text-gray-500" />
                Contact Person *
              </div>
            </Label>
            <Input
              id="contact_person"
              value={formData.contact_person}
              onChange={(e) => setFormData({ ...formData, contact_person: e.target.value })}
              placeholder="John Doe"
              required
            />
          </div>

          {/* Phone */}
          <div className="space-y-2">
            <Label htmlFor="phone">
              <div className="flex items-center gap-2">
                <Phone className="h-4 w-4 text-gray-500" />
                Phone Number
              </div>
            </Label>
            <Input
              id="phone"
              type="tel"
              value={formData.phone}
              onChange={(e) => setFormData({ ...formData, phone: e.target.value })}
              placeholder="+1 (555) 123-4567"
            />
          </div>

          {/* Address */}
          <div className="space-y-2">
            <Label htmlFor="address">
              <div className="flex items-center gap-2">
                <MapPin className="h-4 w-4 text-gray-500" />
                Address
              </div>
            </Label>
            <Input
              id="address"
              value={formData.address}
              onChange={(e) => setFormData({ ...formData, address: e.target.value })}
              placeholder="123 Main St, City, State, ZIP"
            />
          </div>

          {/* License Number */}
          <div className="space-y-2">
            <Label htmlFor="license_number">
              <div className="flex items-center gap-2">
                <FileText className="h-4 w-4 text-gray-500" />
                License Number
              </div>
            </Label>
            <Input
              id="license_number"
              value={formData.license_number}
              onChange={(e) => setFormData({ ...formData, license_number: e.target.value })}
              placeholder="LIC-123456"
            />
          </div>

          {/* Account Status */}
          <div className="pt-4 border-t">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-gray-500" />
                  <Label className="text-base font-medium">Account Status</Label>
                </div>
                <p className="text-sm text-gray-600">
                  {user.is_verified ? (
                    <span className="text-green-600 font-medium">Verified</span>
                  ) : (
                    <span className="text-yellow-600 font-medium">Pending Verification</span>
                  )}
                </p>
              </div>
              <p className="text-xs text-gray-500">
                Member since {new Date(user.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          {/* Save Button */}
          <div className="flex justify-end pt-4">
            <Button onClick={handleSave} disabled={isSaving} size="lg">
              {isSaving ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  Save Changes
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Settings Shortcuts */}
      <Card>
        <CardHeader>
          <CardTitle>Preferences & Settings</CardTitle>
          <CardDescription>
            Configure your notification preferences and other settings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Link href="/dashboard/settings/alerts">
            <Button variant="outline" className="w-full justify-between" size="lg">
              <div className="flex items-center gap-2">
                <Bell className="h-4 w-4" />
                Alert Preferences
              </div>
              <ArrowRight className="h-4 w-4" />
            </Button>
          </Link>
        </CardContent>
      </Card>

      {/* Account Actions */}
      <Card>
        <CardHeader>
          <CardTitle>Account Actions</CardTitle>
          <CardDescription>
            Manage your account and session
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button
            onClick={handleLogout}
            variant="outline"
            className="w-full justify-start"
            size="lg"
          >
            <LogOut className="mr-2 h-4 w-4" />
            Sign Out
          </Button>

          <div className="border-t pt-4">
            <div className="bg-red-50 border border-red-200 rounded-lg p-4 space-y-3">
              <div>
                <h4 className="font-medium text-red-900">Danger Zone</h4>
                <p className="text-sm text-red-700 mt-1">
                  Once you delete your account, there is no going back. Please be certain.
                </p>
              </div>
              <Button
                onClick={handleDeleteAccount}
                variant="destructive"
                className="w-full"
                size="lg"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Delete Account
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
