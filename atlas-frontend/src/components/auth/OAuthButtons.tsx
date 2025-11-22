'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { OAuthService, OAuthProvider, OAuthProviderInfo } from '@/lib/services/oauth-service';
import { Loader2 } from 'lucide-react';

// SVG Icons for OAuth providers (inline to avoid external dependencies)
const GoogleIcon = () => (
  <svg className="w-5 h-5" viewBox="0 0 24 24">
    <path
      fill="#4285F4"
      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
    />
    <path
      fill="#34A853"
      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
    />
    <path
      fill="#FBBC05"
      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
    />
    <path
      fill="#EA4335"
      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
    />
  </svg>
);

const GitHubIcon = ({ className = '' }: { className?: string }) => (
  <svg className={`w-5 h-5 ${className}`} fill="currentColor" viewBox="0 0 24 24">
    <path
      fillRule="evenodd"
      d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
      clipRule="evenodd"
    />
  </svg>
);

const MicrosoftIcon = () => (
  <svg className="w-5 h-5" viewBox="0 0 24 24">
    <path fill="#F25022" d="M1 1h10v10H1z" />
    <path fill="#00A4EF" d="M1 13h10v10H1z" />
    <path fill="#7FBA00" d="M13 1h10v10H13z" />
    <path fill="#FFB900" d="M13 13h10v10H13z" />
  </svg>
);

// Default providers to show even if not configured (Google only)
const DEFAULT_PROVIDERS: Array<{
  name: OAuthProvider;
  displayName: string;
  icon: React.ReactNode;
}> = [
  { name: 'google', displayName: 'Google', icon: <GoogleIcon /> },
];

interface OAuthButtonsProps {
  mode: 'login' | 'register' | 'link';
  onError?: (error: string) => void;
  className?: string;
  showPlaceholders?: boolean;
}

export function OAuthButtons({ mode, onError, className = '', showPlaceholders = true }: OAuthButtonsProps) {
  const [providers, setProviders] = useState<OAuthProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingProvider, setLoadingProvider] = useState<string | null>(null);

  useEffect(() => {
    loadProviders();
  }, []);

  const loadProviders = async () => {
    try {
      const response = await OAuthService.getProviders();
      setProviders(response.providers);
    } catch (error) {
      console.error('Failed to load OAuth providers:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleOAuthClick = async (providerName: string, isConfigured: boolean) => {
    if (!isConfigured) {
      onError?.(`${providerName} sign-in is not yet configured. Please use email/password.`);
      return;
    }

    const provider = providerName as OAuthProvider;
    setLoadingProvider(provider);

    try {
      if (mode === 'link') {
        const response = await OAuthService.startLinkFlow(provider);
        window.open(response.auth_url, '_blank', 'width=500,height=600');
      } else {
        OAuthService.startOAuthFlow(provider);
      }
    } catch (error) {
      console.error('OAuth flow error:', error);
      onError?.(error instanceof Error ? error.message : 'OAuth authentication failed');
      setLoadingProvider(null);
    }
  };

  // While loading, show skeleton
  if (loading) {
    return (
      <div className={`space-y-3 ${className}`}>
        <div className="h-10 bg-gray-100 rounded animate-pulse" />
      </div>
    );
  }

  const actionText = mode === 'login' ? 'Continue' : mode === 'register' ? 'Sign up' : 'Link';

  // Use configured providers or fall back to placeholders
  const enabledProviders = providers.filter(p => p.enabled);
  const hasConfiguredProviders = enabledProviders.length > 0;

  // If no providers and we don't want placeholders, render nothing
  if (!hasConfiguredProviders && !showPlaceholders) {
    return null;
  }

  return (
    <div className={`space-y-3 ${className}`}>
      {hasConfiguredProviders ? (
        // Render configured providers
        enabledProviders.map((provider) => {
          const providerKey = provider.name as OAuthProvider;
          const isLoading = loadingProvider === provider.name;
          const defaultProvider = DEFAULT_PROVIDERS.find(p => p.name === providerKey);
          const icon = defaultProvider?.icon;

          return (
            <Button
              key={provider.name}
              type="button"
              variant="outline"
              className="w-full flex items-center justify-center gap-3 bg-white hover:bg-gray-50 text-gray-700 border border-gray-300 transition-colors"
              onClick={() => handleOAuthClick(provider.name, true)}
              disabled={isLoading || loadingProvider !== null}
            >
              {isLoading ? (
                <Loader2 className="h-5 w-5 animate-spin" />
              ) : (
                icon
              )}
              <span>
                {actionText} with {provider.display_name || provider.name}
              </span>
            </Button>
          );
        })
      ) : (
        // Render placeholder buttons for default providers
        DEFAULT_PROVIDERS.map((provider) => {
          const isLoading = loadingProvider === provider.name;

          return (
            <Button
              key={provider.name}
              type="button"
              variant="outline"
              className="w-full flex items-center justify-center gap-3 bg-white hover:bg-gray-50 text-gray-700 border border-gray-300 transition-colors"
              onClick={() => handleOAuthClick(provider.name, false)}
              disabled={isLoading || loadingProvider !== null}
            >
              {isLoading ? (
                <Loader2 className="h-5 w-5 animate-spin" />
              ) : (
                provider.icon
              )}
              <span>
                {actionText} with {provider.displayName}
              </span>
            </Button>
          );
        })
      )}
    </div>
  );
}

/**
 * Divider component for separating OAuth from email/password login
 */
export function OAuthDivider({ text = 'or continue with email' }: { text?: string }) {
  return (
    <div className="relative my-6">
      <div className="absolute inset-0 flex items-center">
        <div className="w-full border-t border-gray-300" />
      </div>
      <div className="relative flex justify-center text-sm">
        <span className="px-3 bg-white text-gray-500">
          {text}
        </span>
      </div>
    </div>
  );
}
