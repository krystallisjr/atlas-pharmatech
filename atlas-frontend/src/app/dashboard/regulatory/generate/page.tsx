'use client';

import { useState } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Progress } from '@/components/ui/progress';
import {
  FileText,
  FileCheck,
  Sparkles,
  ChevronLeft,
  ChevronRight,
  Shield,
  Database,
  CheckCircle,
  Loader2,
  ArrowRight,
  Eye,
} from 'lucide-react';
import { regulatoryApi } from '@/lib/api';
import {
  DocumentType,
  GenerateDocumentRequest,
  GeneratedDocument,
} from '@/types/regulatory';
import { toast } from 'react-toastify';
import { useRouter } from 'next/navigation';
import { RagContextViewer } from '@/components/regulatory/RagContextViewer';

const DOCUMENT_TYPES: Array<{
  type: DocumentType;
  label: string;
  description: string;
  icon: typeof FileText;
}> = [
  {
    type: 'COA',
    label: 'Certificate of Analysis',
    description: 'Pharmaceutical product testing and quality certification',
    icon: FileCheck,
  },
  {
    type: 'GDP',
    label: 'Good Distribution Practice',
    description: 'Distribution and supply chain compliance documentation',
    icon: FileText,
  },
  {
    type: 'GMP',
    label: 'Good Manufacturing Practice',
    description: 'Manufacturing process and quality assurance records',
    icon: Shield,
  },
];

type GenerationStep = 'select_type' | 'enter_details' | 'generating' | 'preview';

export default function GenerateDocumentPage() {
  const router = useRouter();
  const [step, setStep] = useState<GenerationStep>('select_type');
  const [selectedType, setSelectedType] = useState<DocumentType | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [generatedDoc, setGeneratedDoc] = useState<GeneratedDocument | null>(null);
  const [progress, setProgress] = useState(0);
  const [currentProgressStep, setCurrentProgressStep] = useState('');

  // Form data
  const [productName, setProductName] = useState('');
  const [batchNumber, setBatchNumber] = useState('');
  const [manufacturer, setManufacturer] = useState('');
  const [testResults, setTestResults] = useState('');

  const handleSelectType = (type: DocumentType) => {
    setSelectedType(type);
    setStep('enter_details');
  };

  const simulateProgress = (duration: number) => {
    const steps = [
      { progress: 10, label: 'Connecting to AI engine...' },
      { progress: 20, label: 'Retrieving FDA regulations...' },
      { progress: 40, label: 'Analyzing 10 relevant sections...' },
      { progress: 60, label: 'Generating with Claude AI...' },
      { progress: 75, label: 'Creating Ed25519 signature...' },
      { progress: 90, label: 'Adding to blockchain ledger...' },
      { progress: 100, label: 'Document ready!' },
    ];

    let currentStepIndex = 0;
    const stepDuration = duration / steps.length;

    const interval = setInterval(() => {
      if (currentStepIndex < steps.length) {
        const step = steps[currentStepIndex];
        setProgress(step.progress);
        setCurrentProgressStep(step.label);
        currentStepIndex++;
      } else {
        clearInterval(interval);
      }
    }, stepDuration);

    return interval;
  };

  const handleGenerate = async () => {
    if (!selectedType) return;

    try {
      setIsGenerating(true);
      setStep('generating');
      setProgress(0);

      // Start progress simulation
      const progressInterval = simulateProgress(8000);

      // Prepare request
      const request: GenerateDocumentRequest = {
        document_type: selectedType,
        product_name: productName || undefined,
        batch_number: batchNumber || undefined,
        manufacturer: manufacturer || undefined,
        test_results: testResults ? JSON.parse(testResults) : undefined,
      };

      // Generate document
      const doc = await regulatoryApi.generate(request);

      clearInterval(progressInterval);
      setProgress(100);
      setCurrentProgressStep('Document ready!');
      setGeneratedDoc(doc);

      setTimeout(() => {
        setStep('preview');
        toast.success('Document generated successfully!');
      }, 500);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to generate document');
      setStep('enter_details');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleApprove = async () => {
    if (!generatedDoc) return;

    try {
      await regulatoryApi.approve(generatedDoc.id);
      toast.success('Document approved and added to blockchain!');
      router.push(`/dashboard/regulatory/${generatedDoc.id}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to approve document');
    }
  };

  const renderStepIndicator = () => {
    const steps = [
      { key: 'select_type', label: 'Select Type' },
      { key: 'enter_details', label: 'Enter Details' },
      { key: 'generating', label: 'AI Generation' },
      { key: 'preview', label: 'Preview' },
    ];

    const currentIndex = steps.findIndex(s => s.key === step);

    return (
      <div className="flex items-center justify-center mb-8">
        {steps.map((s, index) => (
          <div key={s.key} className="flex items-center">
            <div
              className={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${
                index <= currentIndex
                  ? 'bg-blue-600 border-blue-600 text-white'
                  : 'bg-white border-gray-300 text-gray-400'
              }`}
            >
              {index < currentIndex ? (
                <CheckCircle className="h-5 w-5" />
              ) : (
                <span>{index + 1}</span>
              )}
            </div>
            <div className="ml-2 mr-4">
              <div
                className={`text-sm font-medium ${
                  index <= currentIndex ? 'text-gray-900' : 'text-gray-400'
                }`}
              >
                {s.label}
              </div>
            </div>
            {index < steps.length - 1 && (
              <ArrowRight className={`h-4 w-4 mr-4 ${index < currentIndex ? 'text-blue-600' : 'text-gray-300'}`} />
            )}
          </div>
        ))}
      </div>
    );
  };

  const renderSelectType = () => (
    <div className="space-y-6">
      <div className="text-center mb-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">Select Document Type</h2>
        <p className="text-gray-600">Choose the type of regulatory document you want to generate</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {DOCUMENT_TYPES.map(({ type, label, description, icon: Icon }) => (
          <Card
            key={type}
            className="cursor-pointer hover:shadow-lg transition-all border-2 hover:border-blue-500"
            onClick={() => handleSelectType(type)}
          >
            <CardContent className="p-6 text-center">
              <div className="mb-4 flex justify-center">
                <div className="p-4 bg-blue-100 rounded-full">
                  <Icon className="h-8 w-8 text-blue-600" />
                </div>
              </div>
              <Badge className="mb-3">{type}</Badge>
              <h3 className="text-lg font-semibold text-gray-900 mb-2">{label}</h3>
              <p className="text-sm text-gray-600">{description}</p>
              <div className="mt-4">
                <Button variant="outline" className="w-full">
                  Select
                  <ChevronRight className="h-4 w-4 ml-2" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );

  const renderEnterDetails = () => {
    const selectedDocType = DOCUMENT_TYPES.find(d => d.type === selectedType);

    return (
      <div className="space-y-6 max-w-2xl mx-auto">
        <div className="text-center mb-8">
          <Badge className="mb-3">{selectedType}</Badge>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">{selectedDocType?.label}</h2>
          <p className="text-gray-600">Enter product information for AI generation</p>
        </div>

        <Card>
          <CardContent className="p-6 space-y-4">
            <div>
              <Label htmlFor="product_name">Product Name</Label>
              <Input
                id="product_name"
                value={productName}
                onChange={(e) => setProductName(e.target.value)}
                placeholder="e.g., Aspirin Tablets 325mg"
                className="mt-1"
              />
            </div>

            <div>
              <Label htmlFor="batch_number">Batch Number</Label>
              <Input
                id="batch_number"
                value={batchNumber}
                onChange={(e) => setBatchNumber(e.target.value)}
                placeholder="e.g., ASP-2025-001"
                className="mt-1"
              />
            </div>

            <div>
              <Label htmlFor="manufacturer">Manufacturer</Label>
              <Input
                id="manufacturer"
                value={manufacturer}
                onChange={(e) => setManufacturer(e.target.value)}
                placeholder="e.g., Atlas Pharma Corp"
                className="mt-1"
              />
            </div>

            <div>
              <Label htmlFor="test_results">Test Results (JSON, optional)</Label>
              <Textarea
                id="test_results"
                value={testResults}
                onChange={(e) => setTestResults(e.target.value)}
                placeholder='{"purity": "99.5%", "dissolution": "Pass"}'
                rows={4}
                className="mt-1 font-mono text-sm"
              />
              <p className="text-xs text-gray-500 mt-1">
                Optional: Enter test results as JSON object
              </p>
            </div>
          </CardContent>
        </Card>

        <Alert className="bg-blue-50 border-blue-200">
          <Database className="h-4 w-4 text-blue-600" />
          <AlertDescription className="text-blue-900">
            <strong>AI + RAG:</strong> Our AI will retrieve relevant FDA/EU/ICH regulations from the knowledge base
            and generate a compliant document based on your product information.
          </AlertDescription>
        </Alert>

        <div className="flex gap-3">
          <Button
            variant="outline"
            onClick={() => setStep('select_type')}
            className="flex-1"
          >
            <ChevronLeft className="h-4 w-4 mr-2" />
            Back
          </Button>
          <Button
            onClick={handleGenerate}
            className="flex-1 bg-blue-600 hover:bg-blue-700"
            disabled={!productName}
          >
            Generate with AI
            <Sparkles className="h-4 w-4 ml-2" />
          </Button>
        </div>
      </div>
    );
  };

  const renderGenerating = () => (
    <div className="space-y-6 max-w-2xl mx-auto">
      <div className="text-center">
        <div className="mb-6 flex justify-center">
          <div className="p-6 bg-blue-100 rounded-full">
            <Loader2 className="h-12 w-12 text-blue-600 animate-spin" />
          </div>
        </div>
        <h2 className="text-2xl font-bold text-gray-900 mb-2">Generating Document...</h2>
        <p className="text-gray-600">AI is creating your regulatory document</p>
      </div>

      <Card>
        <CardContent className="p-8">
          <div className="space-y-6">
            <div>
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-gray-700">{currentProgressStep}</span>
                <span className="text-sm text-gray-500">{progress}%</span>
              </div>
              <Progress value={progress} className="h-2" />
            </div>

            <div className="space-y-3">
              <div className="flex items-center gap-3 text-sm">
                {progress >= 20 ? (
                  <CheckCircle className="h-4 w-4 text-green-600" />
                ) : (
                  <div className="h-4 w-4 rounded-full border-2 border-gray-300" />
                )}
                <span className={progress >= 20 ? 'text-green-600' : 'text-gray-500'}>
                  Retrieving FDA regulations
                </span>
              </div>

              <div className="flex items-center gap-3 text-sm">
                {progress >= 40 ? (
                  <CheckCircle className="h-4 w-4 text-green-600" />
                ) : (
                  <div className="h-4 w-4 rounded-full border-2 border-gray-300" />
                )}
                <span className={progress >= 40 ? 'text-green-600' : 'text-gray-500'}>
                  Analyzing 10 relevant sections
                </span>
              </div>

              <div className="flex items-center gap-3 text-sm">
                {progress >= 60 ? (
                  <CheckCircle className="h-4 w-4 text-green-600" />
                ) : (
                  <div className="h-4 w-4 rounded-full border-2 border-gray-300" />
                )}
                <span className={progress >= 60 ? 'text-green-600' : 'text-gray-500'}>
                  Generating with Claude AI
                </span>
              </div>

              <div className="flex items-center gap-3 text-sm">
                {progress >= 75 ? (
                  <CheckCircle className="h-4 w-4 text-green-600" />
                ) : (
                  <div className="h-4 w-4 rounded-full border-2 border-gray-300" />
                )}
                <span className={progress >= 75 ? 'text-green-600' : 'text-gray-500'}>
                  Creating Ed25519 signature
                </span>
              </div>

              <div className="flex items-center gap-3 text-sm">
                {progress >= 90 ? (
                  <CheckCircle className="h-4 w-4 text-green-600" />
                ) : (
                  <div className="h-4 w-4 rounded-full border-2 border-gray-300" />
                )}
                <span className={progress >= 90 ? 'text-green-600' : 'text-gray-500'}>
                  Adding to blockchain ledger
                </span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );

  const renderPreview = () => {
    if (!generatedDoc) return null;

    return (
      <div className="space-y-6">
        <div className="text-center mb-8">
          <div className="mb-4 flex justify-center">
            <div className="p-4 bg-green-100 rounded-full">
              <CheckCircle className="h-12 w-12 text-green-600" />
            </div>
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">Document Generated Successfully!</h2>
          <p className="text-gray-600">Review your document and approve when ready</p>
        </div>

        <Card className="border-2 border-green-200 bg-green-50">
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Document: {generatedDoc.document_number}</span>
              <Badge variant="default" className="bg-green-600">
                <CheckCircle className="h-3 w-3 mr-1" />
                Cryptographically Signed
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-gray-600">Type:</span>
                <span className="ml-2 font-medium">{generatedDoc.document_type}</span>
              </div>
              <div>
                <span className="text-gray-600">Status:</span>
                <span className="ml-2 font-medium capitalize">{generatedDoc.status}</span>
              </div>
              <div>
                <span className="text-gray-600">Generated By:</span>
                <span className="ml-2 font-medium">{generatedDoc.generated_by}</span>
              </div>
              <div>
                <span className="text-gray-600">Created:</span>
                <span className="ml-2 font-medium">
                  {new Date(generatedDoc.created_at).toLocaleString()}
                </span>
              </div>
            </div>

            <div className="border-t pt-4">
              <h4 className="font-semibold text-gray-900 mb-2">Document Title</h4>
              <p className="text-gray-700">{generatedDoc.title}</p>
            </div>

            <div className="border-t pt-4">
              <h4 className="font-semibold text-gray-900 mb-2">Content Hash (SHA-256)</h4>
              <p className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                {generatedDoc.content_hash}
              </p>
            </div>

            <div className="border-t pt-4">
              <h4 className="font-semibold text-gray-900 mb-2">Ed25519 Signature</h4>
              <p className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                {generatedDoc.generated_signature.substring(0, 64)}...
              </p>
              <p className="text-xs text-gray-500 mt-1">Algorithm: Ed25519 (FIPS 186-4)</p>
            </div>
          </CardContent>
        </Card>

        {/* RAG Context */}
        {generatedDoc.rag_context && generatedDoc.rag_context.length > 0 && (
          <RagContextViewer ragContext={generatedDoc.rag_context} />
        )}

        <div className="flex gap-3">
          <Button
            variant="outline"
            onClick={() => router.push(`/dashboard/regulatory/${generatedDoc.id}`)}
            className="flex-1"
          >
            <Eye className="h-4 w-4 mr-2" />
            View Full Document
          </Button>
          <Button
            onClick={handleApprove}
            className="flex-1 bg-green-600 hover:bg-green-700"
          >
            <CheckCircle className="h-4 w-4 mr-2" />
            Approve Document
          </Button>
        </div>

        <Alert className="bg-blue-50 border-blue-200">
          <Shield className="h-4 w-4 text-blue-600" />
          <AlertDescription className="text-blue-900">
            <strong>Next Step:</strong> Approving this document will create a second signature in the blockchain ledger,
            establishing a complete audit trail.
          </AlertDescription>
        </Alert>
      </div>
    );
  };

  return (
    <DashboardLayout>
      <div className="p-6">
        <div className="max-w-6xl mx-auto space-y-8">
          {/* Header */}
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">Generate Regulatory Document</h1>
              <p className="text-gray-600">AI-powered document generation with FDA/EU/ICH compliance</p>
            </div>
            {step !== 'generating' && (
              <Button
                variant="outline"
                onClick={() => router.push('/dashboard/regulatory')}
              >
                <ChevronLeft className="h-4 w-4 mr-2" />
                Back to Dashboard
              </Button>
            )}
          </div>

          {/* Step Indicator */}
          {renderStepIndicator()}

          {/* Step Content */}
          {step === 'select_type' && renderSelectType()}
          {step === 'enter_details' && renderEnterDetails()}
          {step === 'generating' && renderGenerating()}
          {step === 'preview' && renderPreview()}
        </div>
      </div>
    </DashboardLayout>
  );
}
