'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  FileText,
  ChevronLeft,
  CheckCircle,
  Clock,
  AlertTriangle,
  Download,
  Shield,
  Database,
  Link2,
  Loader2,
  Eye,
} from 'lucide-react';
import { regulatoryApi } from '@/lib/api';
import {
  GeneratedDocument,
  VerificationResult,
} from '@/types/regulatory';
import { toast } from 'react-toastify';
import { useRouter, useParams } from 'next/navigation';
import { RagContextViewer } from '@/components/regulatory/RagContextViewer';
import { BlockchainAuditTrail } from '@/components/regulatory/BlockchainAuditTrail';
import { SignatureVerification } from '@/components/regulatory/SignatureVerification';

export default function DocumentDetailPage() {
  const router = useRouter();
  const params = useParams();
  const documentId = params.id as string;

  const [document, setDocument] = useState<GeneratedDocument | null>(null);
  const [verification, setVerification] = useState<VerificationResult | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isApproving, setIsApproving] = useState(false);

  useEffect(() => {
    if (documentId) {
      loadDocument();
    }
  }, [documentId]);

  const loadDocument = async () => {
    try {
      setIsLoading(true);
      const [docData, verifyResult] = await Promise.all([
        regulatoryApi.getById(documentId),
        regulatoryApi.verify(documentId),
      ]);
      setDocument(docData);
      setVerification(verifyResult);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load document');
      router.push('/dashboard/regulatory');
    } finally {
      setIsLoading(false);
    }
  };

  const handleApprove = async () => {
    if (!document) return;

    try {
      setIsApproving(true);
      await regulatoryApi.approve(document.id);
      toast.success('Document approved successfully!');
      await loadDocument(); // Reload to get updated data
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to approve document');
    } finally {
      setIsApproving(false);
    }
  };

  const handleDownload = () => {
    if (!document) return;

    const content = JSON.stringify(document, null, 2);
    const blob = new Blob([content], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = window.document.createElement('a');
    a.href = url;
    a.download = `${document.document_number}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('Document downloaded');
  };

  const getStatusBadge = (status: string) => {
    const variants = {
      approved: { variant: 'default' as const, icon: CheckCircle, color: 'text-green-600' },
      draft: { variant: 'secondary' as const, icon: Clock, color: 'text-yellow-600' },
      rejected: { variant: 'destructive' as const, icon: AlertTriangle, color: 'text-red-600' },
    };
    const config = variants[status as keyof typeof variants] || variants.draft;
    const Icon = config.icon;
    return (
      <Badge variant={config.variant} className="flex items-center gap-1">
        <Icon className={`h-3 w-3 ${config.color}`} />
        {status.toUpperCase()}
      </Badge>
    );
  };

  if (isLoading) {
    return (
      <DashboardLayout>
        <div className="p-6">
          <div className="flex flex-col items-center justify-center py-12">
            <Loader2 className="h-12 w-12 animate-spin text-blue-600 mb-4" />
            <p className="text-gray-600">Loading document...</p>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  if (!document || !verification) {
    return (
      <DashboardLayout>
        <div className="p-6">
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <AlertDescription>Document not found</AlertDescription>
          </Alert>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between flex-wrap gap-3">
          <div className="flex items-center gap-4">
            <Button
              variant="outline"
              onClick={() => router.push('/dashboard/regulatory')}
            >
              <ChevronLeft className="h-4 w-4 mr-2" />
              Back
            </Button>
            <div>
              <h1 className="text-3xl font-bold text-gray-900">{document.document_number}</h1>
              <p className="text-gray-600">{document.title}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              onClick={handleDownload}
            >
              <Download className="h-4 w-4 mr-2" />
              Download
            </Button>
            {document.status === 'draft' && (
              <Button
                onClick={handleApprove}
                disabled={isApproving}
                className="bg-green-600 hover:bg-green-700"
              >
                {isApproving ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <CheckCircle className="h-4 w-4 mr-2" />
                )}
                Approve Document
              </Button>
            )}
          </div>
        </div>

        {/* Document Info Card */}
        <Card className="border-2 border-blue-200 bg-gradient-to-r from-blue-50 to-indigo-50">
          <CardContent className="p-6">
            <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
              <div>
                <div className="text-sm text-gray-600 mb-1">Document Type</div>
                <Badge className="bg-blue-600 text-white">
                  {document.document_type}
                </Badge>
              </div>
              <div>
                <div className="text-sm text-gray-600 mb-1">Status</div>
                {getStatusBadge(document.status)}
              </div>
              <div>
                <div className="text-sm text-gray-600 mb-1">Generated</div>
                <div className="font-medium text-gray-900">
                  {new Date(document.created_at).toLocaleDateString('en-US', {
                    year: 'numeric',
                    month: 'short',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </div>
              </div>
              <div>
                <div className="text-sm text-gray-600 mb-1">Verification</div>
                {verification.overall_valid ? (
                  <div className="flex items-center gap-1 text-green-600 font-medium">
                    <CheckCircle className="h-4 w-4" />
                    Verified
                  </div>
                ) : (
                  <div className="flex items-center gap-1 text-red-600 font-medium">
                    <AlertTriangle className="h-4 w-4" />
                    Failed
                  </div>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Main Content Tabs */}
        <Tabs defaultValue="content" className="w-full">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="content" className="flex items-center gap-2">
              <FileText className="h-4 w-4" />
              Content
            </TabsTrigger>
            <TabsTrigger value="rag" className="flex items-center gap-2">
              <Database className="h-4 w-4" />
              RAG Context
            </TabsTrigger>
            <TabsTrigger value="signatures" className="flex items-center gap-2">
              <Shield className="h-4 w-4" />
              Signatures
            </TabsTrigger>
            <TabsTrigger value="blockchain" className="flex items-center gap-2">
              <Link2 className="h-4 w-4" />
              Blockchain
            </TabsTrigger>
          </TabsList>

          {/* Content Tab */}
          <TabsContent value="content" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>Document Content</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="prose max-w-none">
                    <pre className="bg-gray-100 p-4 rounded-lg overflow-x-auto text-sm">
                      {JSON.stringify(document.content, null, 2)}
                    </pre>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* User Information */}
            <Card>
              <CardHeader>
                <CardTitle>Document Metadata</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-gray-600">Generated By:</span>
                    <span className="ml-2 font-mono">{document.generated_by}</span>
                  </div>
                  <div>
                    <span className="text-gray-600">Created At:</span>
                    <span className="ml-2">{new Date(document.created_at).toLocaleString()}</span>
                  </div>
                  {document.approved_by && (
                    <>
                      <div>
                        <span className="text-gray-600">Approved By:</span>
                        <span className="ml-2 font-mono">{document.approved_by}</span>
                      </div>
                      <div>
                        <span className="text-gray-600">Approved At:</span>
                        <span className="ml-2">
                          {document.approved_at ? new Date(document.approved_at).toLocaleString() : 'N/A'}
                        </span>
                      </div>
                    </>
                  )}
                  {document.updated_at && (
                    <div>
                      <span className="text-gray-600">Last Updated:</span>
                      <span className="ml-2">{new Date(document.updated_at).toLocaleString()}</span>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          {/* RAG Context Tab */}
          <TabsContent value="rag" className="space-y-6">
            {document.rag_context && document.rag_context.length > 0 ? (
              <RagContextViewer ragContext={document.rag_context} />
            ) : (
              <Card>
                <CardContent className="p-12 text-center">
                  <Database className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                  <h3 className="text-lg font-medium text-gray-900 mb-2">No RAG Context</h3>
                  <p className="text-gray-600">
                    This document was generated without retrieving regulatory context
                  </p>
                </CardContent>
              </Card>
            )}
          </TabsContent>

          {/* Signatures Tab */}
          <TabsContent value="signatures" className="space-y-6">
            <SignatureVerification document={document} verification={verification} />
          </TabsContent>

          {/* Blockchain Tab */}
          <TabsContent value="blockchain" className="space-y-6">
            <BlockchainAuditTrail documentId={document.id} />
          </TabsContent>
        </Tabs>

        {/* Approval Notice */}
        {document.status === 'draft' && (
          <Alert className="bg-yellow-50 border-yellow-200">
            <Clock className="h-4 w-4 text-yellow-600" />
            <AlertDescription className="text-yellow-900">
              <strong>Document Pending Approval:</strong> This document is in draft status. Approve it to add a
              second cryptographic signature to the blockchain ledger and mark it as final.
            </AlertDescription>
          </Alert>
        )}

        {document.status === 'approved' && (
          <Alert className="bg-green-50 border-green-200">
            <CheckCircle className="h-4 w-4 text-green-600" />
            <AlertDescription className="text-green-900">
              <strong>Document Approved:</strong> This document has been cryptographically approved and is
              permanently recorded in the blockchain audit ledger with dual signatures.
            </AlertDescription>
          </Alert>
        )}
      </div>
    </DashboardLayout>
  );
}
