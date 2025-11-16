'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  FileText,
  Plus,
  Search,
  Shield,
  CheckCircle,
  AlertTriangle,
  Clock,
  Database,
  ChevronRight,
  Eye,
  FileCheck,
} from 'lucide-react';
import { regulatoryApi } from '@/lib/api';
import {
  GeneratedDocument,
  KnowledgeBaseStats,
  DocumentType,
  DocumentStatus,
} from '@/types/regulatory';
import { toast } from 'react-toastify';
import { useRouter } from 'next/navigation';

export default function RegulatoryDashboardPage() {
  const router = useRouter();
  const [documents, setDocuments] = useState<GeneratedDocument[]>([]);
  const [stats, setStats] = useState<KnowledgeBaseStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [statusFilter, setStatusFilter] = useState<string>('all');

  useEffect(() => {
    loadData();
  }, [typeFilter, statusFilter]);

  const loadData = async () => {
    try {
      setIsLoading(true);
      const [docsResponse, statsData] = await Promise.all([
        regulatoryApi.list({
          document_type: typeFilter !== 'all' ? (typeFilter as DocumentType) : undefined,
          status: statusFilter !== 'all' ? (statusFilter as DocumentStatus) : undefined,
          page: 1,
          page_size: 100,
        }),
        regulatoryApi.getKnowledgeBaseStats(),
      ]);
      setDocuments(docsResponse.documents);
      setStats(statsData);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load regulatory data');
    } finally {
      setIsLoading(false);
    }
  };

  const filteredDocuments = documents.filter(doc => {
    if (!searchTerm) return true;
    const searchLower = searchTerm.toLowerCase();
    return (
      doc.document_number.toLowerCase().includes(searchLower) ||
      doc.title.toLowerCase().includes(searchLower) ||
      doc.document_type.toLowerCase().includes(searchLower)
    );
  });

  const getStatusBadge = (status: DocumentStatus) => {
    const variants = {
      approved: { variant: 'default' as const, icon: CheckCircle, color: 'text-green-600' },
      draft: { variant: 'secondary' as const, icon: Clock, color: 'text-yellow-600' },
      rejected: { variant: 'destructive' as const, icon: AlertTriangle, color: 'text-red-600' },
    };
    const config = variants[status] || variants.draft;
    const Icon = config.icon;
    return (
      <Badge variant={config.variant} className="flex items-center gap-1">
        <Icon className={`h-3 w-3 ${config.color}`} />
        {status.toUpperCase()}
      </Badge>
    );
  };

  const getDocumentTypeColor = (type: DocumentType) => {
    const colors = {
      COA: 'bg-blue-100 text-blue-800 border-blue-200',
      GDP: 'bg-purple-100 text-purple-800 border-purple-200',
      GMP: 'bg-green-100 text-green-800 border-green-200',
    };
    return colors[type] || colors.COA;
  };

  const getDocumentTypeName = (type: DocumentType) => {
    const names = {
      COA: 'Certificate of Analysis',
      GDP: 'Good Distribution Practice',
      GMP: 'Good Manufacturing Practice',
    };
    return names[type];
  };

  if (isLoading) {
    return (
      <DashboardLayout>
        <div className="p-6">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between flex-wrap gap-3">
          <div>
            <h1 className="text-3xl font-bold text-gray-900">Regulatory AI</h1>
            <p className="text-gray-600">AI-powered regulatory document generation with blockchain-grade security</p>
          </div>
          <Button
            onClick={() => router.push('/dashboard/regulatory/generate')}
            className="bg-blue-600 hover:bg-blue-700"
          >
            <Plus className="h-4 w-4 mr-2" />
            Generate Document
          </Button>
        </div>

        {/* Knowledge Base Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <Card className="bg-gradient-to-br from-blue-50 to-blue-100 border-blue-200">
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-blue-900 flex items-center gap-2">
                <Database className="h-4 w-4" />
                Knowledge Base
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-blue-900">
                {stats?.total_entries || 0}
              </div>
              <p className="text-xs text-blue-700 mt-1">
                FDA/EU/ICH Regulations Loaded
              </p>
            </CardContent>
          </Card>

          <Card className="bg-gradient-to-br from-green-50 to-green-100 border-green-200">
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-green-900 flex items-center gap-2">
                <Shield className="h-4 w-4" />
                Documents Generated
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-green-900">
                {documents.length}
              </div>
              <p className="text-xs text-green-700 mt-1">
                Cryptographically Signed
              </p>
            </CardContent>
          </Card>

          <Card className="bg-gradient-to-br from-purple-50 to-purple-100 border-purple-200">
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium text-purple-900 flex items-center gap-2">
                <CheckCircle className="h-4 w-4" />
                Approved Documents
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-purple-900">
                {documents.filter(d => d.status === 'approved').length}
              </div>
              <p className="text-xs text-purple-700 mt-1">
                Blockchain Verified
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Document Type Stats */}
        {stats && stats.by_document_type.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Regulatory Knowledge Coverage</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {stats.by_document_type.map((typeStats) => (
                  <div
                    key={typeStats.document_type}
                    className="p-4 rounded-lg border bg-gradient-to-r from-gray-50 to-gray-100"
                  >
                    <div className="flex items-center justify-between mb-2">
                      <Badge className={getDocumentTypeColor(typeStats.document_type as DocumentType)}>
                        {typeStats.document_type}
                      </Badge>
                      <FileCheck className="h-4 w-4 text-gray-500" />
                    </div>
                    <div className="text-2xl font-bold text-gray-900">{typeStats.count}</div>
                    <div className="text-sm text-gray-600">
                      {typeStats.unique_sources} unique sources
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Search and Filters */}
        <div className="bg-gradient-to-r from-indigo-50 to-blue-50 rounded-xl p-6 border border-indigo-100">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-xl font-semibold text-gray-900">Document Search</h2>
              <p className="text-sm text-gray-600">Search regulatory documents by number, title, or type</p>
            </div>
            <Search className="h-8 w-8 text-indigo-600" />
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  placeholder="Search document number or title..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="pl-10 bg-white border-gray-200 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
                />
              </div>
            </div>
            <Select value={typeFilter} onValueChange={setTypeFilter}>
              <SelectTrigger className="bg-white border-gray-200 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500">
                <SelectValue placeholder="Filter by type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Types</SelectItem>
                <SelectItem value="COA">Certificate of Analysis</SelectItem>
                <SelectItem value="GDP">Good Distribution Practice</SelectItem>
                <SelectItem value="GMP">Good Manufacturing Practice</SelectItem>
              </SelectContent>
            </Select>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger className="bg-white border-gray-200 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500">
                <SelectValue placeholder="Filter by status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Status</SelectItem>
                <SelectItem value="approved">Approved</SelectItem>
                <SelectItem value="draft">Draft</SelectItem>
                <SelectItem value="rejected">Rejected</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* Documents Table */}
        <Card>
          <CardHeader>
            <CardTitle>Regulatory Documents ({filteredDocuments.length})</CardTitle>
          </CardHeader>
          <CardContent>
            {filteredDocuments.length === 0 ? (
              <div className="text-center py-12">
                <FileText className="h-16 w-16 text-gray-400 mx-auto mb-4" />
                <h3 className="text-lg font-medium text-gray-900 mb-2">
                  {documents.length === 0 ? 'No documents generated yet' : 'No documents match your filters'}
                </h3>
                <p className="text-gray-600 mb-6">
                  {documents.length === 0
                    ? 'Generate your first regulatory document with AI-powered compliance assistance'
                    : 'Try adjusting your search or filters'
                  }
                </p>
                {documents.length === 0 && (
                  <Button onClick={() => router.push('/dashboard/regulatory/generate')}>
                    <Plus className="h-4 w-4 mr-2" />
                    Generate First Document
                  </Button>
                )}
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Document Number</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead>Title</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Generated</TableHead>
                      <TableHead>Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredDocuments.map((doc) => (
                      <TableRow key={doc.id}>
                        <TableCell className="font-mono text-sm font-medium">
                          {doc.document_number}
                        </TableCell>
                        <TableCell>
                          <Badge className={getDocumentTypeColor(doc.document_type)}>
                            {doc.document_type}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <div className="max-w-md">
                            <div className="font-medium text-gray-900">{doc.title}</div>
                            <div className="text-sm text-gray-500">
                              {getDocumentTypeName(doc.document_type)}
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>
                          {getStatusBadge(doc.status)}
                        </TableCell>
                        <TableCell className="text-sm text-gray-600">
                          {new Date(doc.created_at).toLocaleDateString('en-US', {
                            year: 'numeric',
                            month: 'short',
                            day: 'numeric',
                          })}
                        </TableCell>
                        <TableCell>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => router.push(`/dashboard/regulatory/${doc.id}`)}
                            className="flex items-center gap-1"
                          >
                            <Eye className="h-4 w-4" />
                            View Details
                            <ChevronRight className="h-3 w-3" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Security Notice */}
        <Alert className="bg-gradient-to-r from-emerald-50 to-teal-50 border-emerald-200">
          <Shield className="h-4 w-4 text-emerald-600" />
          <AlertDescription className="text-emerald-900">
            <strong>Blockchain-Grade Security:</strong> All documents are cryptographically signed with Ed25519 signatures
            and stored in an immutable audit ledger. Every action is mathematically provable and tamper-evident.
          </AlertDescription>
        </Alert>
      </div>
    </DashboardLayout>
  );
}
