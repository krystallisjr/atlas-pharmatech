'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { OAuthService } from '@/lib/services/oauth-service';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Loader2, CheckCircle, XCircle } from 'lucide-react';
import { toast } from 'react-toastify';

type CallbackState = 'processing' | 'success' | 'error';

export default function OAuthCallbackPage() {
  const router = useRouter();
  const { completeMfaLogin } = useAuth();
  const [state, setState] = useState<CallbackState>('processing');
  const [error, setError] = useState<string | null>(null);
  const [isNewUser, setIsNewUser] = useState(false);

  useEffect(() => {
    handleOAuthCallback();
  }, []);

  const handleOAuthCallback = async () => {
    try {
      // Parse callback parameters
      const result = OAuthService.handleCallback();

      if (!result.success) {
        setState('error');
        setError(result.error || 'OAuth authentication failed');
        return;
      }

      if (!result.token) {
        setState('error');
        setError('No authentication token received');
        return;
      }

      // Complete the OAuth login
      const { user, token } = await OAuthService.completeOAuthLogin(result.token);

      // Update auth context
      completeMfaLogin(user, token);

      setIsNewUser(result.isNewUser || false);
      setState('success');

      // Show success message
      if (result.isNewUser) {
        toast.success('Welcome! Your account has been created.');
      } else {
        toast.success('Login successful!');
      }

      // Redirect to dashboard after short delay
      setTimeout(() => {
        router.push('/dashboard');
      }, 1500);
    } catch (error) {
      console.error('OAuth callback error:', error);
      setState('error');
      setError(error instanceof Error ? error.message : 'Authentication failed');
    }
  };

  const handleRetry = () => {
    router.push('/login');
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle>
            {state === 'processing' && 'Completing Sign In...'}
            {state === 'success' && (isNewUser ? 'Account Created!' : 'Sign In Successful!')}
            {state === 'error' && 'Authentication Failed'}
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col items-center space-y-4">
          {state === 'processing' && (
            <>
              <Loader2 className="h-12 w-12 animate-spin text-blue-600" />
              <p className="text-gray-600 text-center">
                Please wait while we complete your authentication...
              </p>
            </>
          )}

          {state === 'success' && (
            <>
              <CheckCircle className="h-12 w-12 text-green-600" />
              <p className="text-gray-600 text-center">
                {isNewUser
                  ? 'Your account has been created. Redirecting to dashboard...'
                  : 'You are now signed in. Redirecting to dashboard...'}
              </p>
            </>
          )}

          {state === 'error' && (
            <>
              <XCircle className="h-12 w-12 text-red-600" />
              <p className="text-red-600 text-center">{error}</p>
              <div className="flex gap-3 mt-4">
                <Button variant="outline" onClick={handleRetry}>
                  Try Again
                </Button>
                <Button onClick={() => router.push('/')}>
                  Go Home
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
