'use client';

import { useState } from 'react';
import { RefreshCw, Loader2, ArrowRight, ArrowLeft, ArrowLeftRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ErpService } from '@/lib/services';
import type { SyncDirection } from '@/types/erp';
import { toast } from 'react-toastify';

interface SyncTriggerButtonProps {
  connectionId: string;
  onSyncComplete?: () => void;
  variant?: 'default' | 'outline';
  size?: 'default' | 'sm' | 'lg';
  showDirectionSelector?: boolean;
}

export function SyncTriggerButton({
  connectionId,
  onSyncComplete,
  variant = 'default',
  size = 'default',
  showDirectionSelector = false,
}: SyncTriggerButtonProps) {
  const [syncing, setSyncing] = useState(false);
  const [showSelector, setShowSelector] = useState(false);
  const [direction, setDirection] = useState<SyncDirection>('bidirectional');

  const handleTriggerSync = async (selectedDirection?: SyncDirection) => {
    setSyncing(true);

    try {
      const syncDirection = selectedDirection || direction;

      const syncLog = await ErpService.triggerSync(connectionId, {
        direction: syncDirection,
      });

      toast.success(
        `Sync initiated successfully (${getDirectionLabel(syncDirection)})`,
        { autoClose: 3000 }
      );

      // Close selector if open
      setShowSelector(false);

      // Notify parent
      onSyncComplete?.();
    } catch (error: any) {
      const errorMessage = error.response?.data?.error || 'Failed to trigger sync';
      toast.error(errorMessage);
      console.error('Sync trigger error:', error);
    } finally {
      setSyncing(false);
    }
  };

  const getDirectionLabel = (dir: SyncDirection): string => {
    switch (dir) {
      case 'atlas_to_erp':
        return 'Atlas → ERP';
      case 'erp_to_atlas':
        return 'ERP → Atlas';
      case 'bidirectional':
        return 'Bidirectional';
      default:
        return dir;
    }
  };

  const getDirectionIcon = (dir: SyncDirection) => {
    switch (dir) {
      case 'atlas_to_erp':
        return <ArrowRight className="h-4 w-4" />;
      case 'erp_to_atlas':
        return <ArrowLeft className="h-4 w-4" />;
      case 'bidirectional':
        return <ArrowLeftRight className="h-4 w-4" />;
      default:
        return <RefreshCw className="h-4 w-4" />;
    }
  };

  if (showDirectionSelector && showSelector) {
    return (
      <div className="flex items-center gap-2 p-3 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-lg">
        <span className="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
          Sync Direction:
        </span>

        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => handleTriggerSync('atlas_to_erp')}
            disabled={syncing}
            className="gap-2"
          >
            <ArrowRight className="h-4 w-4" />
            Atlas → ERP
          </Button>

          <Button
            size="sm"
            variant="outline"
            onClick={() => handleTriggerSync('erp_to_atlas')}
            disabled={syncing}
            className="gap-2"
          >
            <ArrowLeft className="h-4 w-4" />
            ERP → Atlas
          </Button>

          <Button
            size="sm"
            onClick={() => handleTriggerSync('bidirectional')}
            disabled={syncing}
            className="gap-2"
          >
            <ArrowLeftRight className="h-4 w-4" />
            Bidirectional
          </Button>
        </div>

        <Button
          size="sm"
          variant="ghost"
          onClick={() => setShowSelector(false)}
          disabled={syncing}
        >
          Cancel
        </Button>
      </div>
    );
  }

  return (
    <Button
      variant={variant}
      size={size}
      onClick={() => {
        if (showDirectionSelector) {
          setShowSelector(true);
        } else {
          handleTriggerSync();
        }
      }}
      disabled={syncing}
      className="gap-2"
    >
      {syncing ? (
        <>
          <Loader2 className="h-5 w-5 animate-spin" />
          Syncing...
        </>
      ) : (
        <>
          <RefreshCw className="h-5 w-5" />
          {showDirectionSelector ? 'Trigger Sync' : 'Sync Now'}
        </>
      )}
    </Button>
  );
}
