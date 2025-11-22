'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
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
  ShieldAlert,
  CheckCircle2,
  XCircle,
  Clock,
  Building2,
  Mail,
  Phone,
  MapPin,
  FileText,
  AlertCircle,
  Package,
  Activity,
} from 'lucide-react';
import { AdminService, VerificationQueueItem } from '@/lib/services/admin-service';
import { toast } from 'react-toastify';

export default function VerificationQueuePage() {
  const [queue, setQueue] = useState<VerificationQueueItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Dialog states
  const [showVerifyDialog, setShowVerifyDialog] = useState(false);
  const [selectedUser, setSelectedUser] = useState<VerificationQueueItem | null>(null);
  const [verifyAction, setVerifyAction] = useState<boolean>(true);
  const [verifyNotes, setVerifyNotes] = useState('');
  const [verifyLoading, setVerifyLoading] = useState(false);

  useEffect(() => {
    loadQueue();
  }, []);

  const loadQueue = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await AdminService.getVerificationQueue();
      setQueue(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load verification queue';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const openVerifyDialog = (item: VerificationQueueItem, approve: boolean) => {
    setSelectedUser(item);
    setVerifyAction(approve);
    setShowVerifyDialog(true);
  };

  const handleVerify = async () => {
    if (!selectedUser) return;

    try {
      setVerifyLoading(true);
      await AdminService.verifyUser(
        selectedUser.user.id,
        verifyAction,
        verifyNotes || undefined
      );

      toast.success(`User ${verifyAction ? 'approved' : 'rejected'} successfully`);

      // Remove from queue
      setQueue((prev) => prev.filter((item) => item.user.id !== selectedUser.user.id));

      setShowVerifyDialog(false);
      setVerifyNotes('');
      setSelectedUser(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update verification status';
      toast.error(message);
    } finally {
      setVerifyLoading(false);
    }
  };

  const getDaysWaitingColor = (days: number): string => {
    if (days <= 1) return 'text-green-600';
    if (days <= 3) return 'text-yellow-600';
    if (days <= 7) return 'text-orange-600';
    return 'text-red-600';
  };

  return (
    <DashboardLayout>
      <div className="p-8 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
              <ShieldAlert className="h-8 w-8 text-orange-600" />
              Verification Queue
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              Review and approve pending user verifications
            </p>
          </div>
          <Button onClick={loadQueue} variant="outline">
            Refresh
          </Button>
        </div>

        {/* Queue Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Pending Verifications</p>
                  <p className="text-3xl font-bold text-orange-600 mt-1">{queue.length}</p>
                </div>
                <ShieldAlert className="h-12 w-12 text-orange-600 opacity-20" />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Waiting {'>'} 3 Days</p>
                  <p className="text-3xl font-bold text-red-600 mt-1">
                    {queue.filter((item) => item.days_waiting > 3).length}
                  </p>
                </div>
                <Clock className="h-12 w-12 text-red-600 opacity-20" />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">Waiting {'<'} 24 Hours</p>
                  <p className="text-3xl font-bold text-green-600 mt-1">
                    {queue.filter((item) => item.days_waiting <= 1).length}
                  </p>
                </div>
                <Clock className="h-12 w-12 text-green-600 opacity-20" />
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Queue List */}
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600 dark:text-gray-400">Loading verification queue...</span>
          </div>
        ) : error ? (
          <Card className="border-red-200 dark:border-red-900">
            <CardContent className="pt-6">
              <div className="flex items-center gap-3 text-red-600 dark:text-red-400">
                <AlertCircle className="h-5 w-5" />
                <p className="font-medium">{error}</p>
              </div>
              <Button onClick={loadQueue} className="mt-4" variant="outline">
                Retry
              </Button>
            </CardContent>
          </Card>
        ) : queue.length === 0 ? (
          <Card>
            <CardContent className="py-12">
              <div className="text-center">
                <CheckCircle2 className="h-16 w-16 text-green-600 mx-auto mb-4 opacity-20" />
                <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
                  All Caught Up!
                </h3>
                <p className="text-gray-600 dark:text-gray-400">
                  There are no pending verifications at the moment.
                </p>
              </div>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-4">
            {queue.map((item) => (
              <Card key={item.user.id} className="hover:shadow-lg transition-shadow">
                <CardContent className="p-6">
                  <div className="flex items-start justify-between">
                    {/* User Info */}
                    <div className="flex-1 space-y-4">
                      <div className="flex items-start justify-between">
                        <div>
                          <h3 className="text-xl font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                            {item.user.company_name}
                            <Badge
                              variant="outline"
                              className={`ml-2 ${getDaysWaitingColor(item.days_waiting)}`}
                            >
                              <Clock className="h-3 w-3 mr-1" />
                              {item.days_waiting} {item.days_waiting === 1 ? 'day' : 'days'} waiting
                            </Badge>
                          </h3>
                          <Link
                            href={`/dashboard/admin/users/${item.user.id}`}
                            className="text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400"
                          >
                            View full profile →
                          </Link>
                        </div>
                      </div>

                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <div className="flex items-center gap-2 text-sm">
                            <Mail className="h-4 w-4 text-gray-400" />
                            <span className="text-gray-600 dark:text-gray-400">Email:</span>
                            <span className="font-medium text-gray-900 dark:text-white">
                              {item.user.email}
                            </span>
                          </div>

                          <div className="flex items-center gap-2 text-sm">
                            <Building2 className="h-4 w-4 text-gray-400" />
                            <span className="text-gray-600 dark:text-gray-400">Contact:</span>
                            <span className="font-medium text-gray-900 dark:text-white">
                              {item.user.contact_person}
                            </span>
                          </div>

                          {item.user.phone && (
                            <div className="flex items-center gap-2 text-sm">
                              <Phone className="h-4 w-4 text-gray-400" />
                              <span className="text-gray-600 dark:text-gray-400">Phone:</span>
                              <span className="font-medium text-gray-900 dark:text-white">
                                {item.user.phone}
                              </span>
                            </div>
                          )}
                        </div>

                        <div className="space-y-2">
                          {item.user.license_number && (
                            <div className="flex items-center gap-2 text-sm">
                              <FileText className="h-4 w-4 text-gray-400" />
                              <span className="text-gray-600 dark:text-gray-400">License:</span>
                              <span className="font-medium text-gray-900 dark:text-white">
                                {item.user.license_number}
                              </span>
                            </div>
                          )}

                          {item.user.address && (
                            <div className="flex items-start gap-2 text-sm">
                              <MapPin className="h-4 w-4 text-gray-400 mt-0.5" />
                              <span className="text-gray-600 dark:text-gray-400">Address:</span>
                              <span className="font-medium text-gray-900 dark:text-white">
                                {item.user.address}
                              </span>
                            </div>
                          )}

                          <div className="flex items-center gap-4 text-sm pt-2">
                            <div className="flex items-center gap-2">
                              <Package className="h-4 w-4 text-gray-400" />
                              <span className="text-gray-600 dark:text-gray-400">
                                {item.inventory_count} items
                              </span>
                            </div>
                            <div className="flex items-center gap-2">
                              <Activity className="h-4 w-4 text-gray-400" />
                              <span className="text-gray-600 dark:text-gray-400">
                                {item.transaction_count} transactions
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Action Buttons */}
                    <div className="flex flex-col gap-2 ml-6">
                      <Button
                        onClick={() => openVerifyDialog(item, true)}
                        className="bg-green-600 hover:bg-green-700"
                        size="sm"
                      >
                        <CheckCircle2 className="h-4 w-4 mr-2" />
                        Approve
                      </Button>
                      <Button
                        onClick={() => openVerifyDialog(item, false)}
                        variant="destructive"
                        size="sm"
                      >
                        <XCircle className="h-4 w-4 mr-2" />
                        Reject
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}

        {/* Verify Dialog */}
        <AlertDialog open={showVerifyDialog} onOpenChange={setShowVerifyDialog}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {verifyAction ? 'Approve Verification' : 'Reject Verification'}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {verifyAction ? (
                  <>
                    You are about to approve <strong>{selectedUser?.user.company_name}</strong>.
                    This will grant them full access to the platform.
                  </>
                ) : (
                  <>
                    You are about to reject <strong>{selectedUser?.user.company_name}</strong>.
                    They will remain unverified and have limited access.
                  </>
                )}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <div className="py-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Notes {verifyAction ? '(optional)' : '(recommended)'}
              </label>
              <Textarea
                value={verifyNotes}
                onChange={(e) => setVerifyNotes(e.target.value)}
                placeholder={
                  verifyAction
                    ? 'Add notes about verification approval...'
                    : 'Add reason for rejection (recommended)...'
                }
                rows={3}
              />
            </div>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={verifyLoading}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={handleVerify}
                disabled={verifyLoading}
                className={verifyAction ? 'bg-green-600 hover:bg-green-700' : ''}
              >
                {verifyLoading ? 'Processing...' : verifyAction ? 'Approve' : 'Reject'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </DashboardLayout>
  );
}
