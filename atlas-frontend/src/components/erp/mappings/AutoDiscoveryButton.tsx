'use client';

import { useState } from 'react';
import { Sparkles, Loader2, CheckCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ErpService } from '@/lib/services';
import { toast } from 'react-toastify';

interface AutoDiscoveryButtonProps {
  connectionId: string;
  onDiscoveryComplete: () => void;
  variant?: 'default' | 'outline';
  size?: 'default' | 'sm' | 'lg';
}

export function AutoDiscoveryButton({
  connectionId,
  onDiscoveryComplete,
  variant = 'default',
  size = 'default',
}: AutoDiscoveryButtonProps) {
  const [discovering, setDiscovering] = useState(false);
  const [progress, setProgress] = useState<string>('');

  const handleDiscover = async () => {
    setDiscovering(true);
    setProgress('Analyzing inventory...');

    try {
      // Start discovery
      const response = await ErpService.autoDiscoverMappings(connectionId);

      // Show progress updates
      if (response.suggestions_found > 0) {
        setProgress(`Found ${response.suggestions_found} potential mappings...`);

        // Simulate AI processing time for UX
        await new Promise(resolve => setTimeout(resolve, 1000));

        setProgress('AI analysis complete!');
        await new Promise(resolve => setTimeout(resolve, 500));

        toast.success(
          `Discovery complete! Found ${response.suggestions_found} AI-powered mapping suggestions`,
          { autoClose: 5000 }
        );
      } else {
        toast.info('No new mapping suggestions found. All products may already be mapped.');
      }

      // Notify parent to reload suggestions
      onDiscoveryComplete();
    } catch (error: any) {
      const errorMessage = error.response?.data?.error || 'AI discovery failed';
      toast.error(errorMessage);
      console.error('Discovery error:', error);
    } finally {
      setDiscovering(false);
      setProgress('');
    }
  };

  return (
    <Button
      variant={variant}
      size={size}
      onClick={handleDiscover}
      disabled={discovering}
      className="gap-2"
    >
      {discovering ? (
        <>
          <Loader2 className="h-5 w-5 animate-spin" />
          {progress || 'Discovering...'}
        </>
      ) : (
        <>
          <Sparkles className="h-5 w-5" />
          Auto-Discover with AI
        </>
      )}
    </Button>
  );
}
