'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, ArrowRight, Check, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { ErpService } from '@/lib/services';
import { toast } from 'react-toastify';
import { ERP_SYSTEMS, type ErpType, type CreateConnectionRequest } from '@/types/erp';

// Step components
import { SystemSelectionStep } from '@/components/erp/connections/SystemSelectionStep';
import { NetSuiteConfigStep } from '@/components/erp/connections/NetSuiteConfigStep';
import { SapConfigStep } from '@/components/erp/connections/SapConfigStep';
import { TestConnectionStep } from '@/components/erp/connections/TestConnectionStep';

type WizardStep = 'select_system' | 'configure' | 'test' | 'complete';

interface WizardState {
  step: WizardStep;
  erp_type?: ErpType;
  connection_name: string;
  // NetSuite fields
  netsuite_account_id: string;
  netsuite_consumer_key: string;
  netsuite_consumer_secret: string;
  netsuite_token_id: string;
  netsuite_token_secret: string;
  netsuite_realm: string;
  // SAP fields
  sap_base_url: string;
  sap_client_id: string;
  sap_client_secret: string;
  sap_token_endpoint: string;
  sap_environment: 'cloud' | 'on_premise';
  sap_plant: string;
  sap_company_code: string;
  // Connection ID after creation
  connection_id?: string;
}

export default function NewErpConnectionPage() {
  const router = useRouter();
  const [saving, setSaving] = useState(false);

  const [state, setState] = useState<WizardState>({
    step: 'select_system',
    connection_name: '',
    netsuite_account_id: '',
    netsuite_consumer_key: '',
    netsuite_consumer_secret: '',
    netsuite_token_id: '',
    netsuite_token_secret: '',
    netsuite_realm: '',
    sap_base_url: '',
    sap_client_id: '',
    sap_client_secret: '',
    sap_token_endpoint: '',
    sap_environment: 'cloud',
    sap_plant: '',
    sap_company_code: '',
  });

  const updateState = (updates: Partial<WizardState>) => {
    setState(prev => ({ ...prev, ...updates }));
  };

  const handleSelectSystem = (erp_type: ErpType) => {
    const systemInfo = ERP_SYSTEMS[erp_type];
    updateState({
      erp_type,
      connection_name: `${systemInfo.name} Connection`,
      step: 'configure'
    });
  };

  const handleConfigureComplete = () => {
    updateState({ step: 'test' });
  };

  const handleTestSuccess = async (connectionId: string) => {
    updateState({
      connection_id: connectionId,
      step: 'complete'
    });
  };

  const handleGoBack = () => {
    if (state.step === 'configure') {
      updateState({ step: 'select_system', erp_type: undefined });
    } else if (state.step === 'test') {
      updateState({ step: 'configure' });
    }
  };

  const handleFinish = () => {
    toast.success('ERP connection created successfully!');
    router.push(`/dashboard/erp/${state.connection_id}`);
  };

  const getStepNumber = (): number => {
    switch (state.step) {
      case 'select_system': return 1;
      case 'configure': return 2;
      case 'test': return 3;
      case 'complete': return 4;
    }
  };

  const currentStep = getStepNumber();
  const totalSteps = 4;

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Button
          variant="ghost"
          onClick={() => router.push('/dashboard/erp')}
          className="gap-2 mb-4"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to ERP Integration
        </Button>

        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          Connect ERP System
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Set up NetSuite or SAP integration for automated inventory sync
        </p>
      </div>

      {/* Progress Indicator */}
      <div className="mb-8">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Step {currentStep} of {totalSteps}
          </span>
          <span className="text-sm text-gray-500 dark:text-gray-400">
            {Math.round((currentStep / totalSteps) * 100)}% Complete
          </span>
        </div>

        <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
          <div
            className="bg-blue-600 h-2 rounded-full transition-all duration-500"
            style={{ width: `${(currentStep / totalSteps) * 100}%` }}
          />
        </div>

        <div className="flex justify-between mt-4">
          {[
            { num: 1, label: 'Select System', step: 'select_system' },
            { num: 2, label: 'Configure', step: 'configure' },
            { num: 3, label: 'Test Connection', step: 'test' },
            { num: 4, label: 'Complete', step: 'complete' }
          ].map(({ num, label, step }) => (
            <div key={num} className="flex flex-col items-center">
              <div className={`
                w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors
                ${currentStep > num ? 'bg-green-600 text-white' :
                  currentStep === num ? 'bg-blue-600 text-white' :
                  'bg-gray-300 dark:bg-gray-600 text-gray-600 dark:text-gray-400'}
              `}>
                {currentStep > num ? <Check className="h-4 w-4" /> : num}
              </div>
              <span className="text-xs text-gray-600 dark:text-gray-400 mt-1 text-center max-w-[80px]">
                {label}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Step Content */}
      <Card className="p-8">
        {state.step === 'select_system' && (
          <SystemSelectionStep onSelect={handleSelectSystem} />
        )}

        {state.step === 'configure' && state.erp_type === 'netsuite' && (
          <NetSuiteConfigStep
            state={state}
            onUpdate={updateState}
            onNext={handleConfigureComplete}
            onBack={handleGoBack}
          />
        )}

        {state.step === 'configure' && state.erp_type === 'sap_s4hana' && (
          <SapConfigStep
            state={state}
            onUpdate={updateState}
            onNext={handleConfigureComplete}
            onBack={handleGoBack}
          />
        )}

        {state.step === 'test' && state.erp_type && (
          <TestConnectionStep
            state={state}
            onSuccess={handleTestSuccess}
            onBack={handleGoBack}
          />
        )}

        {state.step === 'complete' && (
          <div className="text-center py-8">
            <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-green-100 dark:bg-green-900 mb-6">
              <Check className="h-10 w-10 text-green-600 dark:text-green-400" />
            </div>

            <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-3">
              Connection Created Successfully!
            </h2>

            <p className="text-gray-600 dark:text-gray-400 mb-8 max-w-md mx-auto">
              Your {state.erp_type === 'netsuite' ? 'NetSuite' : 'SAP'} connection is ready.
              You can now discover inventory mappings and start syncing.
            </p>

            <div className="flex gap-4 justify-center">
              <Button
                variant="outline"
                onClick={() => router.push('/dashboard/erp')}
              >
                Back to Connections
              </Button>

              <Button onClick={handleFinish} className="gap-2">
                Configure Mappings
                <ArrowRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
