'use client';

import { useEffect, useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { AdminSecurityService } from '@/lib/services/admin-security-service';
import { toast } from 'react-toastify';
import { format, formatDistanceToNow } from 'date-fns';
import {
  Key,
  AlertCircle,
  ShieldCheck,
  AlertTriangle,
  Clock,
  CheckCircle2,
  RotateCw,
  History,
  Shield
} from 'lucide-react';
import type { EncryptionStatus } from '@/types/admin-security';

export default function EncryptionKeyPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<EncryptionStatus | null>(null);

  // Rotation dialog
  const [rotationDialog, setRotationDialog] = useState(false);
  const [rotationReason, setRotationReason] = useState('');
  const [rotating, setRotating] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await AdminSecurityService.getEncryptionStatus();
      setData(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load encryption status';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const handleRotateKey = async () => {
    if (!data) return;

    try {
      setRotating(true);
      const newKey = await AdminSecurityService.rotateEncryptionKey(
        rotationReason || undefined
      );

      toast.success(`Encryption key rotated to version ${newKey.key_version}`);
      toast.warning('Remember to update KMS configuration if needed', {
        autoClose: 10000,
      });

      // Reload data
      await loadData();
      setRotationDialog(false);
      setRotationReason('');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to rotate encryption key';
      toast.error(message);
    } finally {
      setRotating(false);
    }
  };

  const getRotationStatusBadge = () => {
    if (!data) return null;

    const { rotation_status, days_until_rotation } = data;

    switch (rotation_status) {
      case 'OVERDUE':
        return (
          <Badge variant="destructive" className="text-base gap-2 px-4 py-2">
            <AlertTriangle className="h-4 w-4" />
            OVERDUE - Rotation Required
          </Badge>
        );
      case 'SOON':
        return (
          <Badge variant="outline" className="text-base gap-2 px-4 py-2 border-orange-300 text-orange-700 dark:border-orange-700 dark:text-orange-400">
            <Clock className="h-4 w-4" />
            Due in {days_until_rotation} days
          </Badge>
        );
      case 'OK':
        return (
          <Badge variant="secondary" className="text-base gap-2 px-4 py-2 bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
            <ShieldCheck className="h-4 w-4" />
            Healthy
          </Badge>
        );
    }
  };

  const getKeyStatusBadge = (status: string, isActive: boolean) => {
    if (isActive) {
      return (
        <Badge variant="default" className="bg-green-600">
          <CheckCircle2 className="h-3 w-3 mr-1" />
          Active
        </Badge>
      );
    }

    if (status === 'Deprecated') {
      return (
        <Badge variant="outline" className="border-orange-300 text-orange-700 dark:border-orange-700 dark:text-orange-400">
          <Clock className="h-3 w-3 mr-1" />
          Deprecated
        </Badge>
      );
    }

    return (
      <Badge variant="secondary">
        <Shield className="h-3 w-3 mr-1" />
        {status}
      </Badge>
    );
  };

  // Loading state
  if (loading) {
    return (
      <DashboardLayout>
        <div className="p-8 space-y-8">
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading encryption key status...</span>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  // Error state
  if (error || !data) {
    return (
      <DashboardLayout>
        <div className="p-8">
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error || 'Failed to load data'}</p>
              </div>
              <Button onClick={loadData} className="mt-4" variant="outline">
                Retry
              </Button>
            </CardContent>
          </Card>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="p-8 space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-3">
              <Key className="h-8 w-8 text-purple-600" />
              Encryption Key Rotation
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-2">
              Manage encryption key lifecycle and monitor rotation schedule
            </p>
          </div>

          <Button
            onClick={() => setRotationDialog(true)}
            disabled={rotating}
            variant={data.rotation_status === 'OVERDUE' ? 'destructive' : 'default'}
            size="lg"
          >
            <RotateCw className={`h-4 w-4 mr-2 ${rotating ? 'animate-spin' : ''}`} />
            {rotating ? 'Rotating...' : 'Rotate Key Now'}
          </Button>
        </div>

        {/* Current Key Status */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Active Key Card */}
          <Card className={`
            ${data.rotation_status === 'OVERDUE' ? 'border-red-300 dark:border-red-900' : ''}
            ${data.rotation_status === 'SOON' ? 'border-orange-300 dark:border-orange-900' : ''}
          `}>
            <CardHeader>
              <CardTitle className="text-lg flex items-center justify-between">
                <span>Active Encryption Key</span>
                {getRotationStatusBadge()}
              </CardTitle>
              <CardDescription>Currently active Data Encryption Key (DEK)</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Key Version</p>
                  <p className="text-2xl font-bold text-gray-900 dark:text-white">
                    v{data.active_key.key_version}
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Age</p>
                  <p className="text-2xl font-bold text-gray-900 dark:text-white">
                    {data.active_key.age_days} days
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Created</p>
                  <p className="text-sm font-medium text-gray-900 dark:text-white">
                    {format(new Date(data.active_key.created_at), 'PPP')}
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Valid Until</p>
                  <p className="text-sm font-medium text-gray-900 dark:text-white">
                    {format(new Date(data.active_key.valid_until), 'PPP')}
                  </p>
                </div>
              </div>

              {data.rotation_status === 'OVERDUE' && (
                <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-900 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400 mt-0.5" />
                    <div>
                      <h4 className="font-semibold text-red-900 dark:text-red-100">
                        Key Rotation Overdue
                      </h4>
                      <p className="text-sm text-red-700 dark:text-red-300 mt-1">
                        This encryption key has exceeded its 90-day rotation period.
                        Please rotate immediately to maintain security standards.
                      </p>
                    </div>
                  </div>
                </div>
              )}

              {data.rotation_status === 'SOON' && (
                <div className="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-900 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <Clock className="h-5 w-5 text-orange-600 dark:text-orange-400 mt-0.5" />
                    <div>
                      <h4 className="font-semibold text-orange-900 dark:text-orange-100">
                        Rotation Recommended
                      </h4>
                      <p className="text-sm text-orange-700 dark:text-orange-300 mt-1">
                        This key will expire in {data.days_until_rotation} days.
                        Consider scheduling a rotation soon.
                      </p>
                    </div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Rotation Info Card */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Rotation Schedule</CardTitle>
              <CardDescription>Automatic rotation policy and timeline</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Rotation Interval</span>
                  <span className="font-semibold text-gray-900 dark:text-white">90 days</span>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Days Until Rotation</span>
                  <span className={`font-semibold ${
                    data.days_until_rotation <= 0 ? 'text-red-600' :
                    data.days_until_rotation <= 7 ? 'text-orange-600' :
                    'text-green-600'
                  }`}>
                    {data.days_until_rotation <= 0 ? 'OVERDUE' : `${data.days_until_rotation} days`}
                  </span>
                </div>
                <div className="flex items-center justify-between py-2 border-b dark:border-gray-700">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Total Keys</span>
                  <span className="font-semibold text-gray-900 dark:text-white">{data.all_keys.length}</span>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Total Rotations</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {data.rotation_history.length}
                  </span>
                </div>
              </div>

              <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-900 rounded-lg p-4 mt-4">
                <div className="flex items-start gap-3">
                  <Shield className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                  <div>
                    <h4 className="font-semibold text-blue-900 dark:text-blue-100 text-sm">
                      Envelope Encryption
                    </h4>
                    <p className="text-xs text-blue-700 dark:text-blue-300 mt-1">
                      Using KEK (Master Key) + DEK (Data Encryption Keys) architecture.
                      Keys are encrypted at rest with the master key from environment.
                    </p>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* All Keys Timeline */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <Key className="h-5 w-5" />
              All Encryption Keys
            </CardTitle>
            <CardDescription>Complete key lifecycle and status</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Version</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead>Age</TableHead>
                    <TableHead>Valid Until</TableHead>
                    <TableHead className="text-right">Days Until Expiry</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.all_keys.map((key) => (
                    <TableRow key={key.id} className={key.is_active ? 'bg-green-50 dark:bg-green-900/10' : ''}>
                      <TableCell className="font-bold">
                        v{key.key_version}
                      </TableCell>
                      <TableCell>
                        {getKeyStatusBadge(key.status, key.is_active)}
                      </TableCell>
                      <TableCell className="text-sm">
                        {format(new Date(key.created_at), 'PPP')}
                      </TableCell>
                      <TableCell className="text-sm">
                        {key.age_days} days
                      </TableCell>
                      <TableCell className="text-sm">
                        {format(new Date(key.valid_until), 'PPP')}
                      </TableCell>
                      <TableCell className="text-right">
                        <Badge
                          variant="secondary"
                          className={
                            key.days_until_expiry <= 0
                              ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                              : key.days_until_expiry <= 7
                              ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                              : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                          }
                        >
                          {key.days_until_expiry <= 0 ? 'Expired' : `${key.days_until_expiry}d`}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>

        {/* Rotation History */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <History className="h-5 w-5" />
              Rotation History
            </CardTitle>
            <CardDescription>Audit trail of all key rotations</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            {data.rotation_history.length === 0 ? (
              <div className="p-8 text-center text-gray-500">
                No rotation history yet
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Date</TableHead>
                      <TableHead>Old Version</TableHead>
                      <TableHead>New Version</TableHead>
                      <TableHead>Rotated By</TableHead>
                      <TableHead>Reason</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.rotation_history.map((event) => (
                      <TableRow key={event.id} className="hover:bg-gray-50 dark:hover:bg-gray-800">
                        <TableCell className="text-sm">
                          <div>
                            {format(new Date(event.rotated_at), 'PPP')}
                          </div>
                          <div className="text-xs text-gray-500 dark:text-gray-400">
                            {formatDistanceToNow(new Date(event.rotated_at), { addSuffix: true })}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">v{event.old_version}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="default" className="bg-green-600">v{event.new_version}</Badge>
                        </TableCell>
                        <TableCell className="text-sm">
                          {event.rotated_by_email || 'System'}
                        </TableCell>
                        <TableCell className="text-sm text-gray-600 dark:text-gray-400">
                          {event.rotation_reason || 'Scheduled rotation'}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Rotation Confirmation Dialog */}
        <AlertDialog open={rotationDialog} onOpenChange={(open) => !rotating && setRotationDialog(open)}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle className="flex items-center gap-2">
                <AlertTriangle className="h-5 w-5 text-orange-600" />
                Rotate Encryption Key
              </AlertDialogTitle>
              <AlertDialogDescription className="space-y-4">
                <p>
                  This will create a new encryption key (v{data.active_key.key_version + 1}) and
                  deprecate the current key (v{data.active_key.key_version}).
                </p>
                <p className="font-semibold text-orange-700 dark:text-orange-400">
                  ⚠️ This is a critical security operation. All new data will be encrypted with the new key.
                  Old keys will remain available for decrypting existing data.
                </p>

                <div className="space-y-2">
                  <Label htmlFor="rotation-reason">Reason for Rotation (Optional)</Label>
                  <Textarea
                    id="rotation-reason"
                    placeholder="e.g., Scheduled 90-day rotation, Security incident response, Compliance requirement..."
                    value={rotationReason}
                    onChange={(e) => setRotationReason(e.target.value)}
                    rows={3}
                    disabled={rotating}
                  />
                </div>
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={rotating}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={handleRotateKey}
                disabled={rotating}
                className="bg-orange-600 hover:bg-orange-700"
              >
                {rotating ? 'Rotating Key...' : 'Confirm Rotation'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </DashboardLayout>
  );
}
