'use client';

import { ArrowRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ERP_SYSTEMS, type ErpType } from '@/types/erp';

interface SystemSelectionStepProps {
  onSelect: (erpType: ErpType) => void;
}

export function SystemSelectionStep({ onSelect }: SystemSelectionStepProps) {
  return (
    <div>
      <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
        Select ERP System
      </h2>
      <p className="text-gray-600 dark:text-gray-400 mb-8">
        Choose which ERP system you want to connect to Atlas Pharma
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* NetSuite Card */}
        <button
          onClick={() => onSelect('netsuite')}
          className="group text-left p-6 border-2 border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-500 dark:hover:border-blue-400 hover:shadow-lg transition-all bg-white dark:bg-gray-800"
        >
          <div className="flex items-start justify-between mb-4">
            <div className="w-12 h-12 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center group-hover:scale-110 transition-transform">
              <svg className="w-8 h-8 text-blue-600 dark:text-blue-400" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 2L2 7v10c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-10-5z"/>
              </svg>
            </div>
            <ArrowRight className="h-5 w-5 text-gray-400 group-hover:text-blue-600 group-hover:translate-x-1 transition-all" />
          </div>

          <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
            {ERP_SYSTEMS.netsuite.name}
          </h3>

          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
            {ERP_SYSTEMS.netsuite.description}
          </p>

          <div className="space-y-2">
            {ERP_SYSTEMS.netsuite.features.map((feature, i) => (
              <div key={i} className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                <div className="w-1.5 h-1.5 rounded-full bg-blue-600" />
                {feature}
              </div>
            ))}
          </div>

          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
            <span className="text-sm font-medium text-blue-600 dark:text-blue-400 group-hover:underline">
              Configure NetSuite →
            </span>
          </div>
        </button>

        {/* SAP Card */}
        <button
          onClick={() => onSelect('sap_s4hana')}
          className="group text-left p-6 border-2 border-gray-200 dark:border-gray-700 rounded-lg hover:border-indigo-500 dark:hover:border-indigo-400 hover:shadow-lg transition-all bg-white dark:bg-gray-800"
        >
          <div className="flex items-start justify-between mb-4">
            <div className="w-12 h-12 rounded-lg bg-indigo-100 dark:bg-indigo-900 flex items-center justify-center group-hover:scale-110 transition-transform">
              <svg className="w-8 h-8 text-indigo-600 dark:text-indigo-400" viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 3h18v18H3V3zm16 16V5H5v14h14zM7 7h10v2H7V7zm0 4h10v2H7v-2zm0 4h10v2H7v-2z"/>
              </svg>
            </div>
            <ArrowRight className="h-5 w-5 text-gray-400 group-hover:text-indigo-600 group-hover:translate-x-1 transition-all" />
          </div>

          <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
            {ERP_SYSTEMS.sap_s4hana.name}
          </h3>

          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
            {ERP_SYSTEMS.sap_s4hana.description}
          </p>

          <div className="space-y-2">
            {ERP_SYSTEMS.sap_s4hana.features.map((feature, i) => (
              <div key={i} className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                <div className="w-1.5 h-1.5 rounded-full bg-indigo-600" />
                {feature}
              </div>
            ))}
          </div>

          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
            <span className="text-sm font-medium text-indigo-600 dark:text-indigo-400 group-hover:underline">
              Configure SAP →
            </span>
          </div>
        </button>
      </div>

      {/* Help Text */}
      <div className="mt-8 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
        <h4 className="font-medium text-gray-900 dark:text-white mb-2">
          Need help choosing?
        </h4>
        <p className="text-sm text-gray-700 dark:text-gray-300">
          Both systems support OAuth authentication, real-time inventory sync, and AI-powered mapping.
          Choose the ERP system you currently use in your organization.
        </p>
      </div>
    </div>
  );
}
