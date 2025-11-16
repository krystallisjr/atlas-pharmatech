import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Link2,
  CheckCircle,
  AlertTriangle,
  Shield,
  Hash,
  Key,
  Clock,
  Loader2,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { regulatoryApi } from '@/lib/api';
import { AuditTrailResponse, VerificationResult } from '@/types/regulatory';
import { toast } from 'react-toastify';

interface BlockchainAuditTrailProps {
  documentId: string;
}

export function BlockchainAuditTrail({ documentId }: BlockchainAuditTrailProps) {
  const [auditTrail, setAuditTrail] = useState<AuditTrailResponse | null>(null);
  const [verification, setVerification] = useState<VerificationResult | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [expandedEntries, setExpandedEntries] = useState<Set<number>>(new Set());

  useEffect(() => {
    loadData();
  }, [documentId]);

  const loadData = async () => {
    try {
      setIsLoading(true);
      const [trail, verifyResult] = await Promise.all([
        regulatoryApi.getAuditTrail(documentId),
        regulatoryApi.verify(documentId),
      ]);
      setAuditTrail(trail);
      setVerification(verifyResult);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load audit trail');
    } finally {
      setIsLoading(false);
    }
  };

  const toggleEntry = (entryId: number) => {
    const newExpanded = new Set(expandedEntries);
    if (newExpanded.has(entryId)) {
      newExpanded.delete(entryId);
    } else {
      newExpanded.add(entryId);
    }
    setExpandedEntries(newExpanded);
  };

  if (isLoading) {
    return (
      <Card>
        <CardContent className="p-8 text-center">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto" />
          <p className="text-gray-600 mt-2">Loading blockchain audit trail...</p>
        </CardContent>
      </Card>
    );
  }

  if (!auditTrail || !verification) {
    return (
      <Alert variant="destructive">
        <AlertTriangle className="h-4 w-4" />
        <AlertDescription>Failed to load audit trail data</AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-6">
      {/* Verification Status */}
      <Card className={`border-2 ${verification.overall_valid ? 'border-green-500 bg-green-50' : 'border-red-500 bg-red-50'}`}>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className={`h-5 w-5 ${verification.overall_valid ? 'text-green-600' : 'text-red-600'}`} />
            Blockchain Verification Status
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="flex items-center gap-2">
              {verification.signature_valid ? (
                <CheckCircle className="h-5 w-5 text-green-600" />
              ) : (
                <AlertTriangle className="h-5 w-5 text-red-600" />
              )}
              <div>
                <div className="text-sm font-medium">Ed25519 Signatures</div>
                <div className={`text-xs ${verification.signature_valid ? 'text-green-600' : 'text-red-600'}`}>
                  {verification.signature_valid ? 'VALID' : 'INVALID'}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {verification.ledger_valid ? (
                <CheckCircle className="h-5 w-5 text-green-600" />
              ) : (
                <AlertTriangle className="h-5 w-5 text-red-600" />
              )}
              <div>
                <div className="text-sm font-medium">Chain Integrity</div>
                <div className={`text-xs ${verification.ledger_valid ? 'text-green-600' : 'text-red-600'}`}>
                  {verification.ledger_valid ? 'VALID' : 'BROKEN'}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {verification.overall_valid ? (
                <CheckCircle className="h-5 w-5 text-green-600" />
              ) : (
                <AlertTriangle className="h-5 w-5 text-red-600" />
              )}
              <div>
                <div className="text-sm font-medium">Overall Status</div>
                <div className={`text-xs font-bold ${verification.overall_valid ? 'text-green-600' : 'text-red-600'}`}>
                  {verification.overall_valid ? 'CRYPTOGRAPHICALLY VERIFIED' : 'VERIFICATION FAILED'}
                </div>
              </div>
            </div>
          </div>

          {verification.overall_valid && (
            <Alert className="bg-green-100 border-green-300">
              <CheckCircle className="h-4 w-4 text-green-600" />
              <AlertDescription className="text-green-900">
                <strong>Mathematically Proven:</strong> This document's signatures and blockchain chain have been
                cryptographically verified. Any tampering would be immediately detectable.
              </AlertDescription>
            </Alert>
          )}

          <div className="text-xs text-gray-600">
            Verified at: {new Date(verification.verified_at).toLocaleString()}
          </div>
        </CardContent>
      </Card>

      {/* Blockchain Chain */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Link2 className="h-5 w-5 text-blue-600" />
            Blockchain Audit Ledger
          </CardTitle>
          <p className="text-sm text-gray-600">
            {auditTrail.total_entries} {auditTrail.total_entries === 1 ? 'entry' : 'entries'} in the immutable chain
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          {auditTrail.ledger_entries.map((entry, index) => {
            const isExpanded = expandedEntries.has(entry.id);
            const isLastEntry = index === auditTrail.ledger_entries.length - 1;

            return (
              <div key={entry.id}>
                <div className="border-2 border-gray-200 rounded-lg p-4 bg-white hover:shadow-md transition-shadow">
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <div className="flex items-center justify-center w-10 h-10 rounded-full bg-blue-100 text-blue-700 font-bold">
                        #{index + 1}
                      </div>
                      <div>
                        <div className="font-semibold text-gray-900">
                          Entry: {entry.operation}
                        </div>
                        <div className="text-xs text-gray-500">
                          <Clock className="h-3 w-3 inline mr-1" />
                          {new Date(entry.created_at).toLocaleString()}
                        </div>
                      </div>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => toggleEntry(entry.id)}
                    >
                      {isExpanded ? (
                        <ChevronUp className="h-4 w-4" />
                      ) : (
                        <ChevronDown className="h-4 w-4" />
                      )}
                    </Button>
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-start gap-2 text-sm">
                      <Hash className="h-4 w-4 text-gray-500 mt-0.5" />
                      <div className="flex-1">
                        <div className="text-gray-600">Chain Hash:</div>
                        <div className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                          {entry.chain_hash}
                        </div>
                      </div>
                    </div>

                    {entry.previous_entry_hash && (
                      <div className="flex items-start gap-2 text-sm">
                        <Link2 className="h-4 w-4 text-blue-500 mt-0.5" />
                        <div className="flex-1">
                          <div className="text-gray-600">Previous Hash:</div>
                          <div className="font-mono text-xs bg-blue-50 p-2 rounded break-all text-blue-800">
                            {entry.previous_entry_hash}
                          </div>
                          <Badge variant="outline" className="mt-1 text-xs">
                            <CheckCircle className="h-3 w-3 mr-1 text-green-600" />
                            Linked to Entry #{index}
                          </Badge>
                        </div>
                      </div>
                    )}

                    {!entry.previous_entry_hash && (
                      <div className="flex items-center gap-2 text-sm">
                        <Badge variant="outline" className="bg-purple-100 text-purple-800 border-purple-300">
                          Genesis Entry (First in Chain)
                        </Badge>
                      </div>
                    )}

                    {isExpanded && (
                      <>
                        <div className="border-t pt-2 mt-2 space-y-2">
                          <div className="flex items-start gap-2 text-sm">
                            <Hash className="h-4 w-4 text-gray-500 mt-0.5" />
                            <div className="flex-1">
                              <div className="text-gray-600">Content Hash:</div>
                              <div className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                                {entry.content_hash}
                              </div>
                            </div>
                          </div>

                          <div className="flex items-start gap-2 text-sm">
                            <Key className="h-4 w-4 text-gray-500 mt-0.5" />
                            <div className="flex-1">
                              <div className="text-gray-600">Signature ({entry.signature_algorithm}):</div>
                              <div className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                                {entry.signature}
                              </div>
                            </div>
                          </div>

                          <div className="flex items-start gap-2 text-sm">
                            <Key className="h-4 w-4 text-gray-500 mt-0.5" />
                            <div className="flex-1">
                              <div className="text-gray-600">Public Key:</div>
                              <div className="font-mono text-xs bg-gray-100 p-2 rounded break-all">
                                {entry.signature_public_key}
                              </div>
                            </div>
                          </div>

                          {entry.metadata && Object.keys(entry.metadata).length > 0 && (
                            <div className="flex items-start gap-2 text-sm">
                              <div className="flex-1">
                                <div className="text-gray-600">Metadata:</div>
                                <pre className="font-mono text-xs bg-gray-100 p-2 rounded overflow-x-auto">
                                  {JSON.stringify(entry.metadata, null, 2)}
                                </pre>
                              </div>
                            </div>
                          )}
                        </div>
                      </>
                    )}
                  </div>
                </div>

                {/* Chain Link Visualization */}
                {!isLastEntry && (
                  <div className="flex justify-center py-2">
                    <div className="flex flex-col items-center">
                      <div className="h-6 w-0.5 bg-blue-400"></div>
                      <Link2 className="h-4 w-4 text-blue-600" />
                      <div className="h-6 w-0.5 bg-blue-400"></div>
                    </div>
                  </div>
                )}
              </div>
            );
          })}

          <Alert className="bg-blue-50 border-blue-200">
            <Shield className="h-4 w-4 text-blue-600" />
            <AlertDescription className="text-blue-900">
              <strong>Blockchain-Style Security:</strong> Each entry's hash includes the previous entry's hash,
              creating an immutable chain. Any modification to a past entry would break the entire chain.
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    </div>
  );
}
