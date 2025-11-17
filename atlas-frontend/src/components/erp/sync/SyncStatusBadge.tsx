'use client';

import { CheckCircle, XCircle, AlertTriangle, Loader2, Clock } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import type { SyncStatus } from '@/types/erp';

interface SyncStatusBadgeProps {
  status: SyncStatus;
  showIcon?: boolean;
}

export function SyncStatusBadge({ status, showIcon = true }: SyncStatusBadgeProps) {
  const getStatusConfig = () => {
    switch (status) {
      case 'completed':
        return {
          label: 'Completed',
          icon: <CheckCircle className="h-3 w-3" />,
          className: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
        };
      case 'failed':
        return {
          label: 'Failed',
          icon: <XCircle className="h-3 w-3" />,
          className: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
        };
      case 'partial':
        return {
          label: 'Partial',
          icon: <AlertTriangle className="h-3 w-3" />,
          className: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
        };
      case 'in_progress':
        return {
          label: 'In Progress',
          icon: <Loader2 className="h-3 w-3 animate-spin" />,
          className: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
        };
      case 'pending':
        return {
          label: 'Pending',
          icon: <Clock className="h-3 w-3" />,
          className: 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200',
        };
      default:
        return {
          label: status,
          icon: null,
          className: 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200',
        };
    }
  };

  const config = getStatusConfig();

  return (
    <Badge className={`${config.className} ${showIcon ? 'gap-1' : ''}`}>
      {showIcon && config.icon}
      {config.label}
    </Badge>
  );
}
