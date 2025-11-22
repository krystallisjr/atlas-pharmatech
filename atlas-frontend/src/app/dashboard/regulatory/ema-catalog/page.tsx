'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import {
  Search,
  Download,
  RefreshCw,
  Database,
  Activity,
  Filter,
  Globe,
  AlertCircle,
  CheckCircle,
  Clock,
  XCircle,
  Settings,
  BarChart3
} from 'lucide-react';
import { EmaService } from '@/lib/services';
import type {
  EmaMedicine,
  EmaStats,
  EmaSyncLog,
  EmaSearchParams,
  EmaRefreshStatus
} from '@/types/ema';
import { toast } from 'react-toastify';

export default function EmaCatalogPage() {
  const [medicines, setMedicines] = useState<EmaMedicine[]>([]);
  const [stats, setStats] = useState<EmaStats | null>(null);
  const [syncLogs, setSyncLogs] = useState<EmaSyncLog[]>([]);
  const [refreshStatus, setRefreshStatus] = useState<EmaRefreshStatus | null>(null);

  // Search and filter state
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedLanguage, setSelectedLanguage] = useState<string>('');
  const [selectedStatus, setSelectedStatus] = useState<string>('');
  const [selectedTherapeuticArea, setSelectedTherapeuticArea] = useState<string>('');

  // Loading states
  const [isLoading, setIsLoading] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isLoadingStats, setIsLoadingStats] = useState(true);

  // Pagination
  const [currentPage, setCurrentPage] = useState(0);
  const [totalResults, setTotalResults] = useState(0);
  const resultsPerPage = 20;

  // Load initial data
  useEffect(() => {
    loadStats();
    loadSyncLogs();
    checkRefreshStatus();

    // Set up periodic refresh for stats
    const interval = setInterval(() => {
      loadStats();
    }, 30000); // Refresh every 30 seconds

    return () => clearInterval(interval);
  }, []);

  // Search when filters change
  useEffect(() => {
    if (stats) { // Only search after initial stats are loaded
      handleSearch();
    }
  }, [searchTerm, selectedLanguage, selectedStatus, selectedTherapeuticArea, currentPage]);

  const loadStats = async () => {
    try {
      setIsLoadingStats(true);
      const statsData = await EmaService.getStats();
      setStats(statsData);
    } catch (error) {
      console.error('Failed to load EMA stats:', error);
      toast.error('Failed to load catalog statistics');
    } finally {
      setIsLoadingStats(false);
    }
  };

  const loadSyncLogs = async () => {
    try {
      const logsData = await EmaService.getSyncLogs(5, 0);
      setSyncLogs(logsData);
    } catch (error) {
      console.error('Failed to load sync logs:', error);
    }
  };

  const checkRefreshStatus = async () => {
    try {
      const statusData = await EmaService.checkRefreshStatus();
      setRefreshStatus(statusData);
    } catch (error) {
      console.error('Failed to check refresh status:', error);
    }
  };

  const handleSearch = async () => {
    try {
      setIsLoading(true);

      const searchParams: EmaSearchParams = {
        query: searchTerm.trim() || undefined,
        language: selectedLanguage || undefined,
        authorization_status: selectedStatus || undefined,
        therapeutic_area: selectedTherapeuticArea || undefined,
        limit: resultsPerPage,
        offset: currentPage * resultsPerPage
      };

      const results = await EmaService.search(searchParams);
      setMedicines(results);

      if (results.length === 0 && searchTerm.trim()) {
        toast.info('No results found. Try different search terms or check if the catalog needs syncing.');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Search failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSync = async () => {
    try {
      setIsSyncing(true);
      toast.info('Starting EMA catalog sync... This may take several minutes.');

      const syncLog = await EmaService.triggerSync({
        language: selectedLanguage || 'en',
        limit: 1000,
        sync_type: 'full'
      });

      toast.success('EMA catalog sync started successfully');
      await loadStats();
      await loadSyncLogs();
      await checkRefreshStatus();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Sync failed');
    } finally {
      setIsSyncing(false);
    }
  };

  const getStatusIcon = (status?: string) => {
    switch (status?.toLowerCase()) {
      case 'authorized':
      case 'active':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'suspended':
      case 'inactive':
        return <Clock className="h-4 w-4 text-yellow-500" />;
      case 'withdrawn':
      case 'refused':
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return <AlertCircle className="h-4 w-4 text-gray-500" />;
    }
  };

  const getSyncStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'failed':
        return <XCircle className="h-4 w-4 text-red-500" />;
      case 'in_progress':
        return <RefreshCw className="h-4 w-4 text-blue-500 animate-spin" />;
      default:
        return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  return (
    <DashboardLayout>
      <div className="space-y-6">
        {/* Header */}
        <div className="flex justify-between items-center">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-2">
              <Globe className="h-8 w-8 text-blue-600" />
              EMA Medicine Catalog
            </h1>
            <p className="text-muted-foreground">
              European Medicines Agency - Centrally authorized medicines
            </p>
          </div>
          <div className="flex gap-2">
            <Button onClick={handleSync} disabled={isSyncing} variant="outline">
              <RefreshCw className={`h-4 w-4 mr-2 ${isSyncing ? 'animate-spin' : ''}`} />
              {isSyncing ? 'Syncing...' : 'Sync EMA Data'}
            </Button>
          </div>
        </div>

        {/* Stats Cards */}
        {stats && (
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Total Medicines</CardTitle>
                <Database className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats.total_entries.toLocaleString()}</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Languages</CardTitle>
                <Globe className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats.entries_by_language.length}</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Orphan Medicines</CardTitle>
                <Activity className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats.orphan_medicines_count.toLocaleString()}</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Last Sync</CardTitle>
                <Clock className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-sm">
                  {stats.last_sync_at ? (
                    <div>
                      <div>{new Date(stats.last_sync_at).toLocaleDateString()}</div>
                      <Badge
                        variant={stats.last_sync_status === 'completed' ? 'default' : 'secondary'}
                        className="mt-1"
                      >
                        {stats.last_sync_status}
                      </Badge>
                    </div>
                  ) : (
                    <span className="text-muted-foreground">Never</span>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        )}

        {/* Refresh Status Alert */}
        {refreshStatus && refreshStatus.needs_refresh && (
          <Card className="border-yellow-200 bg-yellow-50">
            <CardContent className="p-4">
              <div className="flex items-center gap-2">
                <AlertCircle className="h-5 w-5 text-yellow-600" />
                <div className="flex-1">
                  <h3 className="font-semibold text-yellow-800">Catalog Needs Refresh</h3>
                  <p className="text-sm text-yellow-700">
                    The catalog data is {refreshStatus.days_threshold} days old and should be refreshed.
                  </p>
                </div>
                <Button onClick={handleSync} disabled={isSyncing} size="sm" variant="outline">
                  <RefreshCw className={`h-4 w-4 mr-2 ${isSyncing ? 'animate-spin' : ''}`} />
                  Refresh Now
                </Button>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Search and Filters */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center">
              <Search className="h-5 w-5 mr-2" />
              Search EMA Medicines
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {/* Search Input */}
              <div className="flex gap-2">
                <Input
                  placeholder="Search by product name, INN, MAH, or EU number..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                  className="flex-1"
                />
                <Button onClick={handleSearch} disabled={isLoading}>
                  <Search className="h-4 w-4 mr-2" />
                  {isLoading ? 'Searching...' : 'Search'}
                </Button>
              </div>

              {/* Filters */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <label className="text-sm font-medium mb-1 block">Language</label>
                  <Select value={selectedLanguage} onValueChange={setSelectedLanguage}>
                    <SelectTrigger>
                      <SelectValue placeholder="All languages" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">All languages</SelectItem>
                      {EmaService.getSupportedLanguages().map(lang => (
                        <SelectItem key={lang} value={lang}>
                          {EmaService.formatLanguageName(lang)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <label className="text-sm font-medium mb-1 block">Authorization Status</label>
                  <Select value={selectedStatus} onValueChange={setSelectedStatus}>
                    <SelectTrigger>
                      <SelectValue placeholder="All statuses" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">All statuses</SelectItem>
                      <SelectItem value="authorized">Authorized</SelectItem>
                      <SelectItem value="suspended">Suspended</SelectItem>
                      <SelectItem value="withdrawn">Withdrawn</SelectItem>
                      <SelectItem value="refused">Refused</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <label className="text-sm font-medium mb-1 block">Therapeutic Area</label>
                  <Select value={selectedTherapeuticArea} onValueChange={setSelectedTherapeuticArea}>
                    <SelectTrigger>
                      <SelectValue placeholder="All areas" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">All areas</SelectItem>
                      {stats?.entries_by_therapeutic_area.slice(0, 10).map(area => (
                        <SelectItem key={area.therapeutic_area} value={area.therapeutic_area}>
                          {area.therapeutic_area}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Results */}
        {medicines.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle>Search Results</CardTitle>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Product Name</TableHead>
                    <TableHead>INN Name</TableHead>
                    <TableHead>MAH</TableHead>
                    <TableHead>Form</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>EU Number</TableHead>
                    <TableHead>Language</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {medicines.map((medicine) => (
                    <TableRow key={medicine.id}>
                      <TableCell className="font-medium">
                        {medicine.product_name}
                      </TableCell>
                      <TableCell>{medicine.inn_name || '-'}</TableCell>
                      <TableCell>{medicine.mah_name}</TableCell>
                      <TableCell>{medicine.pharmaceutical_form || '-'}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          {getStatusIcon(medicine.authorization_status)}
                          <Badge variant={
                            medicine.authorization_status === 'authorized'
                              ? 'default'
                              : 'secondary'
                          }>
                            {medicine.authorization_status || 'Unknown'}
                          </Badge>
                        </div>
                      </TableCell>
                      <TableCell className="font-mono text-sm">
                        {medicine.eu_number}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          {EmaService.formatLanguageName(medicine.language_code)}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        )}

        {/* Recent Sync Logs */}
        {syncLogs.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center">
                <Activity className="h-5 w-5 mr-2" />
                Recent Sync Activity
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {syncLogs.map((log) => (
                  <div key={log.id} className="flex items-center justify-between p-3 border rounded-lg">
                    <div className="flex items-center gap-3">
                      {getSyncStatusIcon(log.status)}
                      <div>
                        <div className="font-medium">
                          {EmaService.formatSyncStatus(log.status)}
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {log.language_code && `Language: ${log.language_code.toUpperCase()}`}
                          {log.records_fetched && ` • ${log.records_fetched.toLocaleString()} records fetched`}
                        </div>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="text-sm">
                        {EmaService.formatDate(log.sync_started_at)}
                      </div>
                      {log.processing_time_ms && (
                        <div className="text-xs text-muted-foreground">
                          Duration: {EmaService.formatSyncDuration(log.processing_time_ms)}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Empty State */}
        {!isLoading && medicines.length === 0 && stats && (
          <Card>
            <CardContent className="p-12 text-center">
              <Database className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <h3 className="text-lg font-semibold mb-2">No Results Found</h3>
              <p className="text-muted-foreground mb-4">
                {searchTerm.trim()
                  ? 'No medicines match your search criteria. Try different terms or filters.'
                  : 'The EMA catalog appears to be empty. Try syncing data from the EMA API.'
                }
              </p>
              <Button onClick={handleSync} disabled={isSyncing}>
                <RefreshCw className={`h-4 w-4 mr-2 ${isSyncing ? 'animate-spin' : ''}`} />
                Sync EMA Data
              </Button>
            </CardContent>
          </Card>
        )}
      </div>
    </DashboardLayout>
  );
}