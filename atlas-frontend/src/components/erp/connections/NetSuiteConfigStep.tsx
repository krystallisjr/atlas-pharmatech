'use client';

import { ArrowLeft, ArrowRight, HelpCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useState } from 'react';

interface NetSuiteConfigStepProps {
  state: any;
  onUpdate: (updates: any) => void;
  onNext: () => void;
  onBack: () => void;
}

export function NetSuiteConfigStep({ state, onUpdate, onNext, onBack }: NetSuiteConfigStepProps) {
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!state.connection_name.trim()) {
      newErrors.connection_name = 'Connection name is required';
    }

    if (!state.netsuite_account_id.trim()) {
      newErrors.netsuite_account_id = 'Account ID is required';
    }

    if (!state.netsuite_consumer_key.trim()) {
      newErrors.netsuite_consumer_key = 'Consumer Key is required';
    }

    if (!state.netsuite_consumer_secret.trim()) {
      newErrors.netsuite_consumer_secret = 'Consumer Secret is required';
    }

    if (!state.netsuite_token_id.trim()) {
      newErrors.netsuite_token_id = 'Token ID is required';
    }

    if (!state.netsuite_token_secret.trim()) {
      newErrors.netsuite_token_secret = 'Token Secret is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleNext = () => {
    if (validate()) {
      onNext();
    }
  };

  const handleChange = (field: string, value: string) => {
    onUpdate({ [field]: value });
    // Clear error when user starts typing
    if (errors[field]) {
      setErrors(prev => {
        const next = { ...prev };
        delete next[field];
        return next;
      });
    }
  };

  return (
    <div>
      <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
        Configure NetSuite Connection
      </h2>
      <p className="text-gray-600 dark:text-gray-400 mb-8">
        Enter your NetSuite OAuth credentials to enable secure API access
      </p>

      <div className="space-y-6">
        {/* Connection Name */}
        <div>
          <Label htmlFor="connection_name">
            Connection Name <span className="text-red-500">*</span>
          </Label>
          <Input
            id="connection_name"
            value={state.connection_name}
            onChange={(e) => handleChange('connection_name', e.target.value)}
            placeholder="e.g., Production NetSuite"
            className={errors.connection_name ? 'border-red-500' : ''}
          />
          {errors.connection_name && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.connection_name}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            A friendly name to identify this connection
          </p>
        </div>

        {/* Account ID */}
        <div>
          <Label htmlFor="netsuite_account_id" className="flex items-center gap-2">
            Account ID <span className="text-red-500">*</span>
            <a
              href="https://docs.oracle.com/en/cloud/saas/netsuite/ns-online-help/section_1498754928.html"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:text-blue-700"
            >
              <HelpCircle className="h-4 w-4" />
            </a>
          </Label>
          <Input
            id="netsuite_account_id"
            value={state.netsuite_account_id}
            onChange={(e) => handleChange('netsuite_account_id', e.target.value)}
            placeholder="e.g., 1234567"
            className={errors.netsuite_account_id ? 'border-red-500' : ''}
          />
          {errors.netsuite_account_id && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.netsuite_account_id}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Found in Setup → Company → Company Information
          </p>
        </div>

        {/* Consumer Key */}
        <div>
          <Label htmlFor="netsuite_consumer_key">
            Consumer Key <span className="text-red-500">*</span>
          </Label>
          <Input
            id="netsuite_consumer_key"
            value={state.netsuite_consumer_key}
            onChange={(e) => handleChange('netsuite_consumer_key', e.target.value)}
            placeholder="OAuth 1.0 Consumer Key"
            className={errors.netsuite_consumer_key ? 'border-red-500' : ''}
          />
          {errors.netsuite_consumer_key && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.netsuite_consumer_key}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Setup → Integration → Manage Integrations → Create → OAuth 1.0
          </p>
        </div>

        {/* Consumer Secret */}
        <div>
          <Label htmlFor="netsuite_consumer_secret">
            Consumer Secret <span className="text-red-500">*</span>
          </Label>
          <Input
            id="netsuite_consumer_secret"
            type="password"
            value={state.netsuite_consumer_secret}
            onChange={(e) => handleChange('netsuite_consumer_secret', e.target.value)}
            placeholder="OAuth 1.0 Consumer Secret"
            className={errors.netsuite_consumer_secret ? 'border-red-500' : ''}
          />
          {errors.netsuite_consumer_secret && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.netsuite_consumer_secret}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Generated when creating the OAuth 1.0 integration
          </p>
        </div>

        {/* Token ID */}
        <div>
          <Label htmlFor="netsuite_token_id">
            Token ID <span className="text-red-500">*</span>
          </Label>
          <Input
            id="netsuite_token_id"
            value={state.netsuite_token_id}
            onChange={(e) => handleChange('netsuite_token_id', e.target.value)}
            placeholder="Access Token ID"
            className={errors.netsuite_token_id ? 'border-red-500' : ''}
          />
          {errors.netsuite_token_id && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.netsuite_token_id}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Setup → Users/Roles → Access Tokens → New
          </p>
        </div>

        {/* Token Secret */}
        <div>
          <Label htmlFor="netsuite_token_secret">
            Token Secret <span className="text-red-500">*</span>
          </Label>
          <Input
            id="netsuite_token_secret"
            type="password"
            value={state.netsuite_token_secret}
            onChange={(e) => handleChange('netsuite_token_secret', e.target.value)}
            placeholder="Access Token Secret"
            className={errors.netsuite_token_secret ? 'border-red-500' : ''}
          />
          {errors.netsuite_token_secret && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.netsuite_token_secret}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Generated when creating the access token
          </p>
        </div>

        {/* Realm (Optional) */}
        <div>
          <Label htmlFor="netsuite_realm">
            Realm (Optional)
          </Label>
          <Input
            id="netsuite_realm"
            value={state.netsuite_realm}
            onChange={(e) => handleChange('netsuite_realm', e.target.value)}
            placeholder="Leave blank to use Account ID"
          />
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Only required if different from Account ID
          </p>
        </div>

        {/* Security Notice */}
        <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
          <h4 className="font-medium text-yellow-900 dark:text-yellow-200 mb-1 flex items-center gap-2">
            <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd"/>
            </svg>
            Security Notice
          </h4>
          <p className="text-sm text-yellow-800 dark:text-yellow-300">
            Your credentials are encrypted with AES-256-GCM before storage and never logged or exposed in API responses.
          </p>
        </div>
      </div>

      {/* Actions */}
      <div className="flex gap-4 mt-8">
        <Button
          variant="outline"
          onClick={onBack}
          className="gap-2"
        >
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>

        <Button
          onClick={handleNext}
          className="gap-2 flex-1"
        >
          Continue to Test
          <ArrowRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
