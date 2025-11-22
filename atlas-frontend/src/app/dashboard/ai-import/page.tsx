'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
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
  Upload,
  Sparkles,
  CheckCircle,
  AlertTriangle,
  XCircle,
  FileText,
  Brain,
  Zap,
  TrendingUp,
  Clock,
  DollarSign,
  Database,
  RefreshCw,
  Eye,
  Download
} from 'lucide-react';
import { AiImportService } from '@/lib/services/ai-import-service';
import type { AiImportSession, UserQuota, AiImportRowResult } from '@/types/ai-import';
import { toast } from 'react-toastify';
import { cn } from '@/lib/utils';

export default function AiImportPage() {
  const [quota, setQuota] = useState<UserQuota | null>(null);
  const [currentSession, setCurrentSession] = useState<AiImportSession | null>(null);
  const [sessions, setSessions] = useState<AiImportSession[]>([]);
  const [rowResults, setRowResults] = useState<AiImportRowResult[]>([]);
  const [uploading, setUploading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Load quota and sessions on mount
  useEffect(() => {
    loadQuota();
    loadSessions();
  }, []);

  // Poll session status when importing
  useEffect(() => {
    if (currentSession && (currentSession.status === 'analyzing' || currentSession.status === 'importing')) {
      const interval = setInterval(() => {
        refreshCurrentSession();
      }, 2000);
      return () => clearInterval(interval);
    }
  }, [currentSession]);

  const loadQuota = async () => {
    try {
      const data = await AiImportService.getUserQuota();
      setQuota(data);
    } catch (error) {
      console.error('Failed to load quota:', error);
    }
  };

  const loadSessions = async () => {
    try {
      const data = await AiImportService.listSessions({ limit: 10 });
      setSessions(data);
    } catch (error) {
      console.error('Failed to load sessions:', error);
    }
  };

  const refreshCurrentSession = async () => {
    if (!currentSession) return;
    try {
      const updated = await AiImportService.getSession(currentSession.id);
      setCurrentSession(updated);
      if (updated.status === 'completed' || updated.status === 'failed') {
        loadQuota();
        loadSessions();
        if (updated.status === 'completed') {
          loadRowResults(updated.id);
        }
      }
    } catch (error) {
      console.error('Failed to refresh session:', error);
    }
  };

  const loadRowResults = async (sessionId: string) => {
    try {
      const data = await AiImportService.getSessionRows(sessionId, { limit: 100 });
      setRowResults(data);
    } catch (error) {
      console.error('Failed to load row results:', error);
    }
  };

  const handleFileSelect = (file: File) => {
    const validTypes = [
      'text/csv',
      'application/vnd.ms-excel',
      'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
    ];

    if (!validTypes.includes(file.type) && !file.name.endsWith('.csv')) {
      toast.error('Please upload a CSV or Excel file');
      return;
    }

    if (file.size > 50 * 1024 * 1024) {
      toast.error('File size must be less than 50MB');
      return;
    }

    setSelectedFile(file);
  };

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setDragActive(true);
    } else if (e.type === 'dragleave') {
      setDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);

    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      handleFileSelect(e.dataTransfer.files[0]);
    }
  };

  const handleUpload = async () => {
    if (!selectedFile) return;

    setUploading(true);
    try {
      const session = await AiImportService.uploadFile(selectedFile);
      setCurrentSession(session);
      setSelectedFile(null);
      toast.success('File uploaded! AI analysis in progress...');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Upload failed');
    } finally {
      setUploading(false);
    }
  };

  const handleStartImport = async () => {
    if (!currentSession) return;

    setImporting(true);
    try {
      const session = await AiImportService.startImport(currentSession.id);
      setCurrentSession(session);
      toast.success('Import started!');
    } catch (error: any) {
      toast.error(error.response?.data?.error || 'Import failed');
    } finally {
      setImporting(false);
    }
  };

  const getStatusBadge = (status: string) => {
    const variants: Record<string, { color: string; icon: any; label: string }> = {
      analyzing: { color: 'bg-blue-500/10 text-blue-500 border-blue-500/20', icon: Brain, label: 'Analyzing' },
      mapping_review: { color: 'bg-purple-500/10 text-purple-500 border-purple-500/20', icon: Eye, label: 'Review Mapping' },
      importing: { color: 'bg-yellow-500/10 text-yellow-500 border-yellow-500/20', icon: RefreshCw, label: 'Importing' },
      completed: { color: 'bg-green-500/10 text-green-500 border-green-500/20', icon: CheckCircle, label: 'Completed' },
      failed: { color: 'bg-red-500/10 text-red-500 border-red-500/20', icon: XCircle, label: 'Failed' },
    };
    const config = variants[status] || variants.analyzing;
    const Icon = config.icon;
    return (
      <Badge variant="outline" className={cn('gap-1.5', config.color)}>
        <Icon className="h-3 w-3" />
        {config.label}
      </Badge>
    );
  };

  return (
    <DashboardLayout>
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
              <Sparkles className="h-8 w-8 text-purple-500" />
              AI-Powered Import
            </h1>
            <p className="text-muted-foreground mt-1">
              Upload your inventory files and let Claude AI handle the mapping
            </p>
          </div>
        </div>

        {/* Upload Zone */}
        {!currentSession && (
          <Card>
            <CardHeader>
              <CardTitle>Upload File</CardTitle>
              <CardDescription>
                Upload a CSV or Excel file with your pharmaceutical inventory data
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div
                className={cn(
                  'border-2 border-dashed rounded-lg p-12 text-center transition-colors cursor-pointer',
                  dragActive ? 'border-purple-500 bg-purple-500/5' : 'border-gray-300 hover:border-purple-500/50'
                )}
                onDragEnter={handleDrag}
                onDragLeave={handleDrag}
                onDragOver={handleDrag}
                onDrop={handleDrop}
                onClick={() => fileInputRef.current?.click()}
              >
                <input
                  ref={fileInputRef}
                  type="file"
                  className="hidden"
                  accept=".csv,.xlsx,.xls"
                  onChange={(e) => e.target.files && handleFileSelect(e.target.files[0])}
                />

                {selectedFile ? (
                  <div className="space-y-4">
                    <FileText className="h-16 w-16 mx-auto text-purple-500" />
                    <div>
                      <p className="font-semibold text-lg">{selectedFile.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {(selectedFile.size / 1024).toFixed(2)} KB
                      </p>
                    </div>
                    <div className="flex gap-2 justify-center">
                      <Button onClick={handleUpload} disabled={uploading} size="lg">
                        {uploading ? (
                          <>
                            <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                            Uploading...
                          </>
                        ) : (
                          <>
                            <Sparkles className="mr-2 h-4 w-4" />
                            Analyze with AI
                          </>
                        )}
                      </Button>
                      <Button
                        variant="outline"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedFile(null);
                        }}
                      >
                        Cancel
                      </Button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-4">
                    <Upload className="h-16 w-16 mx-auto text-gray-400" />
                    <div>
                      <p className="text-lg font-semibold">Drag & drop your file here</p>
                      <p className="text-sm text-muted-foreground">or click to browse</p>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Supported formats: CSV, Excel (.xlsx, .xls) • Max size: 50MB
                    </p>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Current Session */}
        {currentSession && (
          <Card className="border-2 border-purple-500/30">
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Brain className="h-5 w-5 text-purple-500" />
                  {currentSession.original_filename}
                </CardTitle>
                {getStatusBadge(currentSession.status)}
              </div>
              <CardDescription>
                Session ID: {currentSession.id}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Progress */}
              {currentSession.status === 'importing' && (
                <div>
                  <div className="flex justify-between text-sm mb-2">
                    <span>Import Progress</span>
                    <span className="font-semibold">{currentSession.progress_percentage}%</span>
                  </div>
                  <Progress value={currentSession.progress_percentage} className="h-3" />
                </div>
              )}

              {/* Stats Grid */}
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="bg-slate-50 dark:bg-slate-900 rounded-lg p-4">
                  <div className="text-sm text-muted-foreground mb-1">Total Rows</div>
                  <div className="text-2xl font-bold">{currentSession.total_rows}</div>
                </div>
                <div className="bg-green-50 dark:bg-green-950 rounded-lg p-4">
                  <div className="text-sm text-muted-foreground mb-1">Imported</div>
                  <div className="text-2xl font-bold text-green-600">{currentSession.rows_imported}</div>
                </div>
                <div className="bg-red-50 dark:bg-red-950 rounded-lg p-4">
                  <div className="text-sm text-muted-foreground mb-1">Failed</div>
                  <div className="text-2xl font-bold text-red-600">{currentSession.rows_failed}</div>
                </div>
                <div className="bg-yellow-50 dark:bg-yellow-950 rounded-lg p-4">
                  <div className="text-sm text-muted-foreground mb-1">Flagged</div>
                  <div className="text-2xl font-bold text-yellow-600">{currentSession.rows_flagged}</div>
                </div>
              </div>

              {/* Column Mapping Preview */}
              {currentSession.status === 'mapping_review' && (
                <div className="space-y-4">
                  <Alert className="bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800">
                    <Brain className="h-4 w-4" />
                    <AlertDescription>
                      AI has analyzed your file and suggested the following column mapping.
                      Review and approve to continue.
                    </AlertDescription>
                  </Alert>

                  <div className="border rounded-lg overflow-hidden">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>File Column</TableHead>
                          <TableHead>Maps To</TableHead>
                          <TableHead>Confidence</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {Object.entries(currentSession.suggested_mapping).map(([source, target]) => (
                          <TableRow key={source}>
                            <TableCell className="font-mono text-sm">{source}</TableCell>
                            <TableCell className="font-semibold">{target}</TableCell>
                            <TableCell>
                              <div className="flex items-center gap-2">
                                <Progress
                                  value={currentSession.confidence_scores[source] * 100}
                                  className="h-2 w-24"
                                />
                                <span className="text-sm font-semibold">
                                  {(currentSession.confidence_scores[source] * 100).toFixed(0)}%
                                </span>
                              </div>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </div>

                  <div className="flex gap-2">
                    <Button
                      onClick={handleStartImport}
                      disabled={importing}
                      size="lg"
                      className="flex-1"
                    >
                      {importing ? (
                        <>
                          <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                          Starting Import...
                        </>
                      ) : (
                        <>
                          <CheckCircle className="mr-2 h-4 w-4" />
                          Approve & Start Import
                        </>
                      )}
                    </Button>
                    <Button
                      variant="outline"
                      onClick={() => setCurrentSession(null)}
                      disabled={importing}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              )}

              {/* Completed View */}
              {currentSession.status === 'completed' && (
                <div className="space-y-4">
                  <Alert className="bg-green-50 dark:bg-green-950 border-green-200 dark:border-green-800">
                    <CheckCircle className="h-4 w-4" />
                    <AlertDescription>
                      Import completed successfully! {currentSession.rows_imported} items imported.
                    </AlertDescription>
                  </Alert>

                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <span className="text-muted-foreground">NDC Validated:</span>
                      <span className="ml-2 font-semibold">{currentSession.ndc_validated}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Auto-Enriched:</span>
                      <span className="ml-2 font-semibold">{currentSession.auto_enriched}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground">AI Cost:</span>
                      <span className="ml-2 font-semibold">${currentSession.ai_cost_usd}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Duration:</span>
                      <span className="ml-2 font-semibold">
                        {currentSession.completed_at && new Date(currentSession.completed_at).getTime() - new Date(currentSession.created_at).getTime() > 0
                          ? `${Math.round((new Date(currentSession.completed_at).getTime() - new Date(currentSession.created_at).getTime()) / 1000)}s`
                          : 'N/A'}
                      </span>
                    </div>
                  </div>

                  {/* Row Results */}
                  {rowResults.length > 0 && (
                    <div className="border rounded-lg overflow-hidden">
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>Row</TableHead>
                            <TableHead>Status</TableHead>
                            <TableHead>Product</TableHead>
                            <TableHead>Warnings</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {rowResults.slice(0, 10).map((row) => (
                            <TableRow key={row.id}>
                              <TableCell className="font-mono">#{row.row_number}</TableCell>
                              <TableCell>
                                {row.status === 'imported' && (
                                  <Badge variant="outline" className="bg-green-500/10 text-green-500 border-green-500/20">
                                    <CheckCircle className="h-3 w-3 mr-1" />
                                    Imported
                                  </Badge>
                                )}
                                {row.status === 'flagged_for_review' && (
                                  <Badge variant="outline" className="bg-yellow-500/10 text-yellow-500 border-yellow-500/20">
                                    <AlertTriangle className="h-3 w-3 mr-1" />
                                    Flagged
                                  </Badge>
                                )}
                                {row.status === 'failed' && (
                                  <Badge variant="outline" className="bg-red-500/10 text-red-500 border-red-500/20">
                                    <XCircle className="h-3 w-3 mr-1" />
                                    Failed
                                  </Badge>
                                )}
                              </TableCell>
                              <TableCell className="font-medium">
                                {row.mapped_data?.brand_name || row.source_data.brand_name || 'N/A'}
                              </TableCell>
                              <TableCell className="text-sm text-muted-foreground">
                                {row.validation_warnings.length > 0 ? (
                                  <span className="flex items-center gap-1">
                                    <AlertTriangle className="h-3 w-3 text-yellow-500" />
                                    {row.validation_warnings.length} warning(s)
                                  </span>
                                ) : (
                                  <span className="text-green-600">No warnings</span>
                                )}
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </div>
                  )}

                  <Button onClick={() => setCurrentSession(null)} variant="outline" className="w-full">
                    <Upload className="mr-2 h-4 w-4" />
                    Import Another File
                  </Button>
                </div>
              )}

              {/* Failed View */}
              {currentSession.status === 'failed' && currentSession.error_message && (
                <Alert variant="destructive">
                  <XCircle className="h-4 w-4" />
                  <AlertDescription>{currentSession.error_message}</AlertDescription>
                </Alert>
              )}
            </CardContent>
          </Card>
        )}

        {/* Recent Sessions */}
        {sessions.length > 0 && !currentSession && (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Clock className="h-5 w-5" />
                Recent Imports
              </CardTitle>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Filename</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Rows</TableHead>
                    <TableHead>Imported</TableHead>
                    <TableHead>Cost</TableHead>
                    <TableHead>Date</TableHead>
                    <TableHead>Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sessions.map((session) => (
                    <TableRow key={session.id}>
                      <TableCell className="font-medium">{session.original_filename}</TableCell>
                      <TableCell>{getStatusBadge(session.status)}</TableCell>
                      <TableCell>{session.total_rows}</TableCell>
                      <TableCell className="text-green-600 font-semibold">{session.rows_imported}</TableCell>
                      <TableCell>${session.ai_cost_usd}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {new Date(session.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            setCurrentSession(session);
                            if (session.status === 'completed') {
                              loadRowResults(session.id);
                            }
                          }}
                        >
                          <Eye className="h-4 w-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        )}
      </div>
    </DashboardLayout>
  );
}
