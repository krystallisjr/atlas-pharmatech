'use client';

import { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import Link from 'next/link';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  User as UserIcon,
  Mail,
  Building2,
  Phone,
  MapPin,
  FileText,
  Calendar,
  Shield,
  CheckCircle2,
  XCircle,
  ArrowLeft,
  Trash2,
  UserCog,
  AlertTriangle,
} from 'lucide-react';
import { AdminService } from '@/lib/services/admin-service';
import { User, UserRole } from '@/types/auth';
import { toast } from 'react-toastify';
import { format } from 'date-fns';
import { useAuth } from '@/contexts/auth-context';

export default function UserDetailsPage() {
  const params = useParams();
  const router = useRouter();
  const { user: currentUser, isSuperadmin } = useAuth();
  const userId = params.id as string;

  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Action states
  const [verifyLoading, setVerifyLoading] = useState(false);
  const [verifyNotes, setVerifyNotes] = useState('');
  const [showVerifyDialog, setShowVerifyDialog] = useState(false);
  const [verifyAction, setVerifyAction] = useState<boolean>(true);

  const [roleLoading, setRoleLoading] = useState(false);
  const [newRole, setNewRole] = useState<UserRole>('user');
  const [roleNotes, setRoleNotes] = useState('');
  const [showRoleDialog, setShowRoleDialog] = useState(false);

  const [deleteLoading, setDeleteLoading] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  useEffect(() => {
    loadUser();
  }, [userId]);

  const loadUser = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await AdminService.getUser(userId);
      setUser(data);
      setNewRole(data.role);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load user';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    if (!user) return;

    try {
      setVerifyLoading(true);
      const updatedUser = await AdminService.verifyUser(
        userId,
        verifyAction,
        verifyNotes || undefined
      );
      setUser(updatedUser);
      toast.success(`User ${verifyAction ? 'verified' : 'unverified'} successfully`);
      setShowVerifyDialog(false);
      setVerifyNotes('');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update verification status';
      toast.error(message);
    } finally {
      setVerifyLoading(false);
    }
  };

  const handleChangeRole = async () => {
    if (!user) return;

    try {
      setRoleLoading(true);
      const updatedUser = await AdminService.changeUserRole(
        userId,
        newRole,
        roleNotes || undefined
      );
      setUser(updatedUser);
      toast.success(`User role changed to ${newRole}`);
      setShowRoleDialog(false);
      setRoleNotes('');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to change user role';
      toast.error(message);
    } finally {
      setRoleLoading(false);
    }
  };

  const handleDelete = async () => {
    try {
      setDeleteLoading(true);
      await AdminService.deleteUser(userId);
      toast.success('User deleted successfully');
      router.push('/dashboard/admin/users');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to delete user';
      toast.error(message);
      setDeleteLoading(false);
    }
  };

  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading user details...</span>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  if (error || !user) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertTriangle className="h-5 w-5" />
                <p className="font-medium">{error || 'User not found'}</p>
              </div>
              <Link href="/dashboard/admin/users">
                <Button className="mt-4" variant="outline">
                  <ArrowLeft className="h-4 w-4 mr-2" />
                  Back to Users
                </Button>
              </Link>
            </CardContent>
          </Card>
        </div>
      </DashboardLayout>
    );
  }

  const isCurrentUser = currentUser?.id === user.id;
  const canChangeRole = isSuperadmin() && !isCurrentUser;
  const canDelete = isSuperadmin() && !isCurrentUser;

  return (
    <DashboardLayout>
      <div className="p-8 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link href="/dashboard/admin/users">
              <Button variant="outline" size="sm">
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back
              </Button>
            </Link>
            <div>
              <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
                User Details
              </h1>
              <p className="text-gray-600 dark:text-gray-400 mt-1">
                View and manage user information
              </p>
            </div>
          </div>
          <Button onClick={loadUser} variant="outline">
            Refresh
          </Button>
        </div>

        {/* User Info Card */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <UserIcon className="h-5 w-5" />
                User Information
              </CardTitle>
              <div className="flex items-center gap-2">
                {user.is_verified ? (
                  <Badge variant="default" className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                    <CheckCircle2 className="h-3 w-3 mr-1" />
                    Verified
                  </Badge>
                ) : (
                  <Badge variant="outline" className="border-orange-300 text-orange-700 dark:border-orange-700 dark:text-orange-400">
                    <XCircle className="h-3 w-3 mr-1" />
                    Unverified
                  </Badge>
                )}
                <Badge variant={user.role === 'superadmin' ? 'destructive' : user.role === 'admin' ? 'default' : 'secondary'}>
                  {user.role === 'superadmin' && <Shield className="h-3 w-3 mr-1" />}
                  {user.role}
                </Badge>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div className="flex items-start gap-3">
                  <Building2 className="h-5 w-5 text-gray-400 mt-0.5" />
                  <div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">Company Name</p>
                    <p className="font-medium text-gray-900 dark:text-white">{user.company_name}</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <Mail className="h-5 w-5 text-gray-400 mt-0.5" />
                  <div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">Email</p>
                    <p className="font-medium text-gray-900 dark:text-white">{user.email}</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <UserIcon className="h-5 w-5 text-gray-400 mt-0.5" />
                  <div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">Contact Person</p>
                    <p className="font-medium text-gray-900 dark:text-white">{user.contact_person}</p>
                  </div>
                </div>
              </div>

              <div className="space-y-4">
                {user.phone && (
                  <div className="flex items-start gap-3">
                    <Phone className="h-5 w-5 text-gray-400 mt-0.5" />
                    <div>
                      <p className="text-sm text-gray-600 dark:text-gray-400">Phone</p>
                      <p className="font-medium text-gray-900 dark:text-white">{user.phone}</p>
                    </div>
                  </div>
                )}

                {user.address && (
                  <div className="flex items-start gap-3">
                    <MapPin className="h-5 w-5 text-gray-400 mt-0.5" />
                    <div>
                      <p className="text-sm text-gray-600 dark:text-gray-400">Address</p>
                      <p className="font-medium text-gray-900 dark:text-white">{user.address}</p>
                    </div>
                  </div>
                )}

                {user.license_number && (
                  <div className="flex items-start gap-3">
                    <FileText className="h-5 w-5 text-gray-400 mt-0.5" />
                    <div>
                      <p className="text-sm text-gray-600 dark:text-gray-400">License Number</p>
                      <p className="font-medium text-gray-900 dark:text-white">{user.license_number}</p>
                    </div>
                  </div>
                )}

                <div className="flex items-start gap-3">
                  <Calendar className="h-5 w-5 text-gray-400 mt-0.5" />
                  <div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">Created At</p>
                    <p className="font-medium text-gray-900 dark:text-white">
                      {format(new Date(user.created_at), 'PPpp')}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Actions Card */}
        <Card>
          <CardHeader>
            <CardTitle>Admin Actions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Verify/Unverify */}
              <Button
                onClick={() => {
                  setVerifyAction(!user.is_verified);
                  setShowVerifyDialog(true);
                }}
                variant={user.is_verified ? 'outline' : 'default'}
                className="w-full"
              >
                {user.is_verified ? (
                  <>
                    <XCircle className="h-4 w-4 mr-2" />
                    Unverify User
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="h-4 w-4 mr-2" />
                    Verify User
                  </>
                )}
              </Button>

              {/* Change Role (Superadmin only) */}
              <Button
                onClick={() => setShowRoleDialog(true)}
                variant="outline"
                className="w-full"
                disabled={!canChangeRole}
              >
                <UserCog className="h-4 w-4 mr-2" />
                Change Role
                {!isSuperadmin() && <Shield className="h-3 w-3 ml-2 text-gray-400" />}
              </Button>

              {/* Delete User (Superadmin only) */}
              <Button
                onClick={() => setShowDeleteDialog(true)}
                variant="destructive"
                className="w-full"
                disabled={!canDelete}
              >
                <Trash2 className="h-4 w-4 mr-2" />
                Delete User
                {!isSuperadmin() && <Shield className="h-3 w-3 ml-2 text-gray-400" />}
              </Button>
            </div>

            {isCurrentUser && (
              <p className="text-sm text-gray-500 dark:text-gray-500 mt-4 text-center">
                You cannot change your own role or delete your own account.
              </p>
            )}
          </CardContent>
        </Card>

        {/* Verify Dialog */}
        <AlertDialog open={showVerifyDialog} onOpenChange={setShowVerifyDialog}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {verifyAction ? 'Verify User' : 'Unverify User'}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {verifyAction
                  ? 'This will mark the user as verified and grant them full access to the platform.'
                  : 'This will revoke the user\'s verified status and may restrict their access.'}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <div className="py-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Notes (optional)
              </label>
              <Textarea
                value={verifyNotes}
                onChange={(e) => setVerifyNotes(e.target.value)}
                placeholder="Add notes about this verification change..."
                rows={3}
              />
            </div>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={verifyLoading}>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={handleVerify} disabled={verifyLoading}>
                {verifyLoading ? 'Processing...' : 'Confirm'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>

        {/* Change Role Dialog */}
        <AlertDialog open={showRoleDialog} onOpenChange={setShowRoleDialog}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Change User Role</AlertDialogTitle>
              <AlertDialogDescription>
                Change the user's role to grant or revoke administrative privileges.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <div className="py-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  New Role
                </label>
                <Select value={newRole} onValueChange={(value) => setNewRole(value as UserRole)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="user">User</SelectItem>
                    <SelectItem value="admin">Admin</SelectItem>
                    <SelectItem value="superadmin">Superadmin</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Notes (optional)
                </label>
                <Textarea
                  value={roleNotes}
                  onChange={(e) => setRoleNotes(e.target.value)}
                  placeholder="Add notes about this role change..."
                  rows={3}
                />
              </div>
            </div>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={roleLoading}>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={handleChangeRole} disabled={roleLoading || newRole === user.role}>
                {roleLoading ? 'Processing...' : 'Change Role'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>

        {/* Delete Dialog */}
        <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle className="flex items-center gap-2 text-red-600">
                <AlertTriangle className="h-5 w-5" />
                Delete User - Irreversible
              </AlertDialogTitle>
              <AlertDialogDescription>
                This action cannot be undone. This will permanently delete the user account and all associated data.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <div className="py-4 bg-red-50 dark:bg-red-900/20 p-4 rounded-md">
              <p className="text-sm text-red-800 dark:text-red-200 font-medium">
                You are about to delete:
              </p>
              <ul className="mt-2 text-sm text-red-700 dark:text-red-300 list-disc list-inside">
                <li>{user.company_name}</li>
                <li>{user.email}</li>
                <li>All user inventory and transaction data</li>
              </ul>
            </div>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={deleteLoading}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={handleDelete}
                disabled={deleteLoading}
                className="bg-red-600 hover:bg-red-700"
              >
                {deleteLoading ? 'Deleting...' : 'Delete Permanently'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </DashboardLayout>
  );
}
