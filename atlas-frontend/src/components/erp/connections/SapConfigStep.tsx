'use client';

import { ArrowLeft, ArrowRight, HelpCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useState } from 'react';

interface SapConfigStepProps {
  state: any;
  onUpdate: (updates: any) => void;
  onNext: () => void;
  onBack: () => void;
}

export function SapConfigStep({ state, onUpdate, onNext, onBack }: SapConfigStepProps) {
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!state.connection_name.trim()) {
      newErrors.connection_name = 'Connection name is required';
    }

    if (!state.sap_base_url.trim()) {
      newErrors.sap_base_url = 'Base URL is required';
    } else if (!state.sap_base_url.startsWith('https://')) {
      newErrors.sap_base_url = 'Base URL must start with https://';
    }

    if (!state.sap_client_id.trim()) {
      newErrors.sap_client_id = 'Client ID is required';
    }

    if (!state.sap_client_secret.trim()) {
      newErrors.sap_client_secret = 'Client Secret is required';
    }

    if (!state.sap_token_endpoint.trim()) {
      newErrors.sap_token_endpoint = 'Token Endpoint is required';
    } else if (!state.sap_token_endpoint.startsWith('https://')) {
      newErrors.sap_token_endpoint = 'Token Endpoint must start with https://';
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
        Configure SAP S/4HANA Connection
      </h2>
      <p className="text-gray-600 dark:text-gray-400 mb-8">
        Enter your SAP OAuth 2.0 credentials to enable OData API access
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
            placeholder="e.g., Production SAP S/4HANA"
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

        {/* Environment */}
        <div>
          <Label htmlFor="sap_environment">
            Environment Type <span className="text-red-500">*</span>
          </Label>
          <Select
            value={state.sap_environment}
            onValueChange={(value) => handleChange('sap_environment', value)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="cloud">SAP S/4HANA Cloud</SelectItem>
              <SelectItem value="on_premise">SAP S/4HANA On-Premise</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Select whether you're using Cloud or On-Premise deployment
          </p>
        </div>

        {/* Base URL */}
        <div>
          <Label htmlFor="sap_base_url" className="flex items-center gap-2">
            Base URL <span className="text-red-500">*</span>
            <a
              href="https://help.sap.com/docs/SAP_S4HANA_CLOUD/0f69f8fb28ac4bf48d2b57b9637e81fa/cee9181c28454f8d956a94f30c39d92f.html"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:text-blue-700"
            >
              <HelpCircle className="h-4 w-4" />
            </a>
          </Label>
          <Input
            id="sap_base_url"
            value={state.sap_base_url}
            onChange={(e) => handleChange('sap_base_url', e.target.value)}
            placeholder={state.sap_environment === 'cloud'
              ? "https://my123456-api.s4hana.cloud.sap"
              : "https://my-sap-server.company.com:443"}
            className={errors.sap_base_url ? 'border-red-500' : ''}
          />
          {errors.sap_base_url && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.sap_base_url}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {state.sap_environment === 'cloud'
              ? 'Your SAP S/4HANA Cloud API endpoint'
              : 'Your SAP S/4HANA server URL with port'}
          </p>
        </div>

        {/* Client ID */}
        <div>
          <Label htmlFor="sap_client_id">
            OAuth Client ID <span className="text-red-500">*</span>
          </Label>
          <Input
            id="sap_client_id"
            value={state.sap_client_id}
            onChange={(e) => handleChange('sap_client_id', e.target.value)}
            placeholder="OAuth 2.0 Client ID"
            className={errors.sap_client_id ? 'border-red-500' : ''}
          />
          {errors.sap_client_id && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.sap_client_id}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Communication Arrangements → OAuth 2.0 Client Credentials
          </p>
        </div>

        {/* Client Secret */}
        <div>
          <Label htmlFor="sap_client_secret">
            OAuth Client Secret <span className="text-red-500">*</span>
          </Label>
          <Input
            id="sap_client_secret"
            type="password"
            value={state.sap_client_secret}
            onChange={(e) => handleChange('sap_client_secret', e.target.value)}
            placeholder="OAuth 2.0 Client Secret"
            className={errors.sap_client_secret ? 'border-red-500' : ''}
          />
          {errors.sap_client_secret && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.sap_client_secret}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Generated when creating the OAuth client
          </p>
        </div>

        {/* Token Endpoint */}
        <div>
          <Label htmlFor="sap_token_endpoint">
            Token Endpoint <span className="text-red-500">*</span>
          </Label>
          <Input
            id="sap_token_endpoint"
            value={state.sap_token_endpoint}
            onChange={(e) => handleChange('sap_token_endpoint', e.target.value)}
            placeholder={state.sap_environment === 'cloud'
              ? "https://my123456.authentication.sap.hana.ondemand.com/oauth/token"
              : "https://my-sap-server.company.com/sap/bc/sec/oauth2/token"}
            className={errors.sap_token_endpoint ? 'border-red-500' : ''}
          />
          {errors.sap_token_endpoint && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-1">
              {errors.sap_token_endpoint}
            </p>
          )}
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            OAuth 2.0 token endpoint URL for authentication
          </p>
        </div>

        {/* Plant (Optional) */}
        <div>
          <Label htmlFor="sap_plant">
            Plant Code (Optional)
          </Label>
          <Input
            id="sap_plant"
            value={state.sap_plant}
            onChange={(e) => handleChange('sap_plant', e.target.value)}
            placeholder="e.g., 1000"
          />
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Default plant for inventory operations (can be left blank)
          </p>
        </div>

        {/* Company Code (Optional) */}
        <div>
          <Label htmlFor="sap_company_code">
            Company Code (Optional)
          </Label>
          <Input
            id="sap_company_code"
            value={state.sap_company_code}
            onChange={(e) => handleChange('sap_company_code', e.target.value)}
            placeholder="e.g., 1000"
          />
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Default company code for transactions (can be left blank)
          </p>
        </div>

        {/* Security Notice */}
        <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
          <h4 className="font-medium text-blue-900 dark:text-blue-200 mb-1">
            OAuth 2.0 Security
          </h4>
          <p className="text-sm text-blue-800 dark:text-blue-300">
            Atlas uses OAuth 2.0 client credentials flow with automatic token refresh.
            Credentials are encrypted with AES-256-GCM encryption at rest.
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
