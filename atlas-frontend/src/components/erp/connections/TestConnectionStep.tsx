'use client';

import { useState, useEffect } from 'react';
import { ArrowLeft, Check, X, Loader2, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ErpService } from '@/lib/services';
import type { CreateConnectionRequest } from '@/types/erp';
import { toast } from 'react-toastify';

interface TestConnectionStepProps {
  state: any;
  onSuccess: (connectionId: string) => void;
  onBack: () => void;
}

type TestPhase = 'idle' | 'saving' | 'testing' | 'success' | 'error';

export function TestConnectionStep({ state, onSuccess, onBack }: TestConnectionStepProps) {
  const [phase, setPhase] = useState<TestPhase>('idle');
  const [connectionId, setConnectionId] = useState<string>('');
  const [error, setError] = useState<string>('');
  const [testResults, setTestResults] = useState<{
    api_reachable?: boolean;
    authentication_valid?: boolean;
    permissions_verified?: boolean;
  }>({});

  const handleTest = async () => {
    try {
      // Phase 1: Save connection
      setPhase('saving');
      setError('');

      const request: CreateConnectionRequest = {
        connection_name: state.connection_name,
        erp_type: state.erp_type,
      };

      // Add type-specific fields
      if (state.erp_type === 'netsuite') {
        request.netsuite_account_id = state.netsuite_account_id;
        request.netsuite_consumer_key = state.netsuite_consumer_key;
        request.netsuite_consumer_secret = state.netsuite_consumer_secret;
        request.netsuite_token_id = state.netsuite_token_id;
        request.netsuite_token_secret = state.netsuite_token_secret;
        if (state.netsuite_realm) {
          request.netsuite_realm = state.netsuite_realm;
        }
      } else if (state.erp_type === 'sap_s4hana') {
        request.sap_base_url = state.sap_base_url;
        request.sap_client_id = state.sap_client_id;
        request.sap_client_secret = state.sap_client_secret;
        request.sap_token_endpoint = state.sap_token_endpoint;
        request.sap_environment = state.sap_environment;
        if (state.sap_plant) {
          request.sap_plant = state.sap_plant;
        }
        if (state.sap_company_code) {
          request.sap_company_code = state.sap_company_code;
        }
      }

      const connection = await ErpService.createConnection(request);
      setConnectionId(connection.id);

      // Phase 2: Test connection
      setPhase('testing');

      const testResult = await ErpService.testConnection(connection.id);

      if (testResult.success) {
        setTestResults(testResult.details || {});
        setPhase('success');

        // Call parent success handler after brief delay to show success state
        setTimeout(() => {
          onSuccess(connection.id);
        }, 1500);
      } else {
        setPhase('error');
        setError(testResult.message || 'Connection test failed');
        toast.error('Connection test failed: ' + testResult.message);
      }
    } catch (err: any) {
      setPhase('error');
      const errorMessage = err.response?.data?.error || err.message || 'Failed to create connection';
      setError(errorMessage);
      toast.error(errorMessage);
    }
  };

  const handleRetry = () => {
    setPhase('idle');
    setError('');
    setTestResults({});
  };

  const getPhaseIcon = (checkPhase: 'save' | 'test') => {
    if (checkPhase === 'save') {
      if (phase === 'saving') return <Loader2 className="h-5 w-5 animate-spin text-blue-600" />;
      if (phase === 'testing' || phase === 'success') return <Check className="h-5 w-5 text-green-600" />;
      if (phase === 'error' && !connectionId) return <X className="h-5 w-5 text-red-600" />;
      return <div className="h-5 w-5 rounded-full border-2 border-gray-300" />;
    } else {
      if (phase === 'testing') return <Loader2 className="h-5 w-5 animate-spin text-blue-600" />;
      if (phase === 'success') return <Check className="h-5 w-5 text-green-600" />;
      if (phase === 'error' && connectionId) return <X className="h-5 w-5 text-red-600" />;
      return <div className="h-5 w-5 rounded-full border-2 border-gray-300" />;
    }
  };

  const getPhaseText = (checkPhase: 'save' | 'test') => {
    if (checkPhase === 'save') {
      if (phase === 'saving') return 'Saving connection...';
      if (phase === 'testing' || phase === 'success') return 'Connection saved';
      if (phase === 'error' && !connectionId) return 'Save failed';
      return 'Save connection to database';
    } else {
      if (phase === 'testing') return 'Testing connection...';
      if (phase === 'success') return 'Connection successful';
      if (phase === 'error' && connectionId) return 'Test failed';
      return 'Verify API credentials';
    }
  };

  return (
    <div>
      <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
        Test Connection
      </h2>
      <p className="text-gray-600 dark:text-gray-400 mb-8">
        We'll save your connection and verify it can reach the ERP API
      </p>

      {/* Connection Summary */}
      <div className="mb-8 p-6 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
        <h3 className="font-semibold text-gray-900 dark:text-white mb-4">
          Connection Summary
        </h3>

        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-gray-600 dark:text-gray-400">Name:</span>
            <span className="font-medium text-gray-900 dark:text-white">
              {state.connection_name}
            </span>
          </div>

          <div className="flex justify-between">
            <span className="text-gray-600 dark:text-gray-400">ERP System:</span>
            <span className="font-medium text-gray-900 dark:text-white">
              {state.erp_type === 'netsuite' ? 'NetSuite' : 'SAP S/4HANA'}
            </span>
          </div>

          {state.erp_type === 'netsuite' && (
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">Account ID:</span>
              <span className="font-medium text-gray-900 dark:text-white">
                {state.netsuite_account_id}
              </span>
            </div>
          )}

          {state.erp_type === 'sap_s4hana' && (
            <>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Environment:</span>
                <span className="font-medium text-gray-900 dark:text-white capitalize">
                  {state.sap_environment.replace('_', ' ')}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Base URL:</span>
                <span className="font-medium text-gray-900 dark:text-white truncate max-w-xs">
                  {state.sap_base_url}
                </span>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Test Progress */}
      <div className="space-y-4 mb-8">
        {/* Save Phase */}
        <div className={`
          p-4 rounded-lg border-2 transition-all
          ${phase === 'saving' ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' :
            (phase === 'testing' || phase === 'success') ? 'border-green-500 bg-green-50 dark:bg-green-900/20' :
            (phase === 'error' && !connectionId) ? 'border-red-500 bg-red-50 dark:bg-red-900/20' :
            'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800'}
        `}>
          <div className="flex items-center gap-3">
            {getPhaseIcon('save')}
            <div className="flex-1">
              <h4 className="font-medium text-gray-900 dark:text-white">
                Step 1: {getPhaseText('save')}
              </h4>
              {connectionId && (
                <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                  Connection ID: {connectionId.substring(0, 8)}...
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Test Phase */}
        <div className={`
          p-4 rounded-lg border-2 transition-all
          ${phase === 'testing' ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' :
            phase === 'success' ? 'border-green-500 bg-green-50 dark:bg-green-900/20' :
            (phase === 'error' && connectionId) ? 'border-red-500 bg-red-50 dark:bg-red-900/20' :
            'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800'}
        `}>
          <div className="flex items-center gap-3">
            {getPhaseIcon('test')}
            <div className="flex-1">
              <h4 className="font-medium text-gray-900 dark:text-white">
                Step 2: {getPhaseText('test')}
              </h4>
              {phase === 'success' && testResults && (
                <div className="mt-2 space-y-1">
                  {testResults.api_reachable !== undefined && (
                    <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                      {testResults.api_reachable ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-red-600" />
                      )}
                      API Reachable
                    </div>
                  )}
                  {testResults.authentication_valid !== undefined && (
                    <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                      {testResults.authentication_valid ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-red-600" />
                      )}
                      Authentication Valid
                    </div>
                  )}
                  {testResults.permissions_verified !== undefined && (
                    <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                      {testResults.permissions_verified ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <X className="h-4 w-4 text-red-600" />
                      )}
                      Permissions Verified
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 rounded-lg border border-red-200 dark:border-red-800">
          <div className="flex gap-3">
            <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="font-medium text-red-900 dark:text-red-200 mb-1">
                Test Failed
              </h4>
              <p className="text-sm text-red-800 dark:text-red-300">
                {error}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-4">
        <Button
          variant="outline"
          onClick={onBack}
          disabled={phase === 'saving' || phase === 'testing'}
          className="gap-2"
        >
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>

        {phase === 'idle' && (
          <Button
            onClick={handleTest}
            className="gap-2 flex-1"
          >
            <Check className="h-4 w-4" />
            Save & Test Connection
          </Button>
        )}

        {phase === 'error' && (
          <Button
            onClick={handleRetry}
            variant="destructive"
            className="gap-2 flex-1"
          >
            Retry Test
          </Button>
        )}

        {(phase === 'saving' || phase === 'testing') && (
          <Button
            disabled
            className="gap-2 flex-1"
          >
            <Loader2 className="h-4 w-4 animate-spin" />
            {phase === 'saving' ? 'Saving...' : 'Testing...'}
          </Button>
        )}

        {phase === 'success' && (
          <Button
            disabled
            className="gap-2 flex-1 bg-green-600 hover:bg-green-700"
          >
            <Check className="h-4 w-4" />
            Connection Successful
          </Button>
        )}
      </div>
    </div>
  );
}
