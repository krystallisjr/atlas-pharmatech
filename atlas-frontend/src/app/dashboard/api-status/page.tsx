'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
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
  Activity,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  Server,
  Clock,
  Zap,
  Database,
  Shield,
  Package,
  Store,
  Users,
  FileText
} from 'lucide-react';
import axios from 'axios';

interface EndpointStatus {
  name: string;
  method: string;
  path: string;
  status: 'online' | 'offline' | 'checking';
  responseTime?: number;
  lastChecked?: Date;
  category: string;
  description: string;
  requiresAuth: boolean;
}

export default function APIStatusPage() {
  const [endpoints, setEndpoints] = useState<EndpointStatus[]>([]);
  const [isChecking, setIsChecking] = useState(false);
  const [lastCheckTime, setLastCheckTime] = useState<Date | null>(null);
  const [apiBaseUrl] = useState(process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080');

  const endpointsList: Omit<EndpointStatus, 'status' | 'responseTime' | 'lastChecked'>[] = [
    // Auth Endpoints
    { name: 'User Registration', method: 'POST', path: '/api/auth/register', category: 'Authentication', description: 'Register new user', requiresAuth: false },
    { name: 'User Login', method: 'POST', path: '/api/auth/login', category: 'Authentication', description: 'User authentication', requiresAuth: false },
    { name: 'Get Profile', method: 'GET', path: '/api/auth/profile', category: 'Authentication', description: 'Get user profile', requiresAuth: true },

    // Pharmaceutical Endpoints
    { name: 'Search Pharmaceuticals', method: 'GET', path: '/api/pharmaceuticals/search', category: 'Pharmaceuticals', description: 'Search pharmaceutical catalog', requiresAuth: true },
    { name: 'Get Manufacturers', method: 'GET', path: '/api/pharmaceuticals/manufacturers', category: 'Pharmaceuticals', description: 'List all manufacturers', requiresAuth: true },
    { name: 'Get Categories', method: 'GET', path: '/api/pharmaceuticals/categories', category: 'Pharmaceuticals', description: 'List all categories', requiresAuth: true },

    // Inventory Endpoints
    { name: 'Get User Inventory', method: 'GET', path: '/api/inventory/my', category: 'Inventory', description: 'Get current user inventory', requiresAuth: true },
    { name: 'Add Inventory', method: 'POST', path: '/api/inventory/', category: 'Inventory', description: 'Add new inventory item', requiresAuth: true },

    // Marketplace Endpoints
    { name: 'Get Marketplace Listings', method: 'GET', path: '/api/marketplace/listings', category: 'Marketplace', description: 'Browse marketplace', requiresAuth: true },
    { name: 'Get Buyer Inquiries', method: 'GET', path: '/api/marketplace/inquiries/buyer', category: 'Marketplace', description: 'Get buyer inquiries', requiresAuth: true },
    { name: 'Get Seller Inquiries', method: 'GET', path: '/api/marketplace/inquiries/seller', category: 'Marketplace', description: 'Get seller inquiries', requiresAuth: true },
    { name: 'Get Transactions', method: 'GET', path: '/api/marketplace/transactions', category: 'Marketplace', description: 'Get transaction history', requiresAuth: true },
  ];

  useEffect(() => {
    checkAllEndpoints();
  }, []);

  const checkEndpoint = async (endpoint: Omit<EndpointStatus, 'status' | 'responseTime' | 'lastChecked'>): Promise<EndpointStatus> => {
    const startTime = performance.now();

    try {
      // For endpoints that require auth, we'll just check if the server responds
      // We can't actually test auth endpoints without valid credentials
      const url = `${apiBaseUrl}${endpoint.path}`;

      const response = await axios.request({
        method: endpoint.method,
        url: url,
        timeout: 5000,
        validateStatus: (status) => {
          // Consider 401/403 as "online" since the server is responding
          return status < 500 || status === 401 || status === 403;
        },
      });

      const endTime = performance.now();
      const responseTime = Math.round(endTime - startTime);

      return {
        ...endpoint,
        status: 'online',
        responseTime,
        lastChecked: new Date(),
      };
    } catch (error) {
      return {
        ...endpoint,
        status: 'offline',
        lastChecked: new Date(),
      };
    }
  };

  const checkAllEndpoints = async () => {
    setIsChecking(true);

    // Set all to checking state
    setEndpoints(endpointsList.map(ep => ({ ...ep, status: 'checking' as const })));

    const results = await Promise.all(
      endpointsList.map(endpoint => checkEndpoint(endpoint))
    );

    setEndpoints(results);
    setLastCheckTime(new Date());
    setIsChecking(false);
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'Authentication':
        return Shield;
      case 'Pharmaceuticals':
        return Package;
      case 'Inventory':
        return Database;
      case 'Marketplace':
        return Store;
      default:
        return FileText;
    }
  };

  const getStatusBadge = (status: EndpointStatus['status']) => {
    switch (status) {
      case 'online':
        return <Badge className="bg-green-100 text-green-800 hover:bg-green-200"><CheckCircle2 className="h-3 w-3 mr-1" /> Online</Badge>;
      case 'offline':
        return <Badge variant="destructive"><XCircle className="h-3 w-3 mr-1" /> Offline</Badge>;
      case 'checking':
        return <Badge variant="secondary"><RefreshCw className="h-3 w-3 mr-1 animate-spin" /> Checking</Badge>;
    }
  };

  const getResponseTimeBadge = (responseTime?: number) => {
    if (!responseTime) return null;

    if (responseTime < 100) {
      return <Badge variant="outline" className="text-green-600 border-green-300">⚡ {responseTime}ms</Badge>;
    } else if (responseTime < 500) {
      return <Badge variant="outline" className="text-yellow-600 border-yellow-300">🟡 {responseTime}ms</Badge>;
    } else {
      return <Badge variant="outline" className="text-orange-600 border-orange-300">🟠 {responseTime}ms</Badge>;
    }
  };

  const groupedEndpoints = endpoints.reduce((acc, endpoint) => {
    if (!acc[endpoint.category]) {
      acc[endpoint.category] = [];
    }
    acc[endpoint.category].push(endpoint);
    return acc;
  }, {} as Record<string, EndpointStatus[]>);

  const onlineCount = endpoints.filter(ep => ep.status === 'online').length;
  const offlineCount = endpoints.filter(ep => ep.status === 'offline').length;
  const avgResponseTime = endpoints
    .filter(ep => ep.responseTime)
    .reduce((sum, ep) => sum + (ep.responseTime || 0), 0) / endpoints.filter(ep => ep.responseTime).length;

  return (
    <DashboardLayout>
      <div className="p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between flex-wrap gap-3">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-3">
              <Activity className="h-8 w-8 text-blue-600" />
              API Status Monitor
            </h1>
            <p className="text-gray-600">Monitor the health and performance of backend API endpoints</p>
          </div>
          <Button onClick={checkAllEndpoints} disabled={isChecking}>
            <RefreshCw className={`h-4 w-4 mr-2 ${isChecking ? 'animate-spin' : ''}`} />
            {isChecking ? 'Checking...' : 'Refresh Status'}
          </Button>
        </div>

        {/* Last Check Time */}
        {lastCheckTime && (
          <Alert>
            <Clock className="h-4 w-4" />
            <AlertDescription>
              Last checked: {lastCheckTime.toLocaleTimeString()} on {lastCheckTime.toLocaleDateString()}
            </AlertDescription>
          </Alert>
        )}

        {/* Overview Stats */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">API Base URL</CardTitle>
              <Server className="h-4 w-4 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-sm font-mono break-all">{apiBaseUrl}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Online Endpoints</CardTitle>
              <CheckCircle2 className="h-4 w-4 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold text-green-600">{onlineCount}/{endpoints.length}</div>
              <p className="text-xs text-muted-foreground">
                {endpoints.length > 0 ? Math.round((onlineCount / endpoints.length) * 100) : 0}% operational
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Offline Endpoints</CardTitle>
              <XCircle className="h-4 w-4 text-red-600" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold text-red-600">{offlineCount}</div>
              <p className="text-xs text-muted-foreground">
                {offlineCount > 0 ? 'Requires attention' : 'All systems operational'}
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Avg Response Time</CardTitle>
              <Zap className="h-4 w-4 text-yellow-600" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {avgResponseTime ? Math.round(avgResponseTime) : '--'}ms
              </div>
              <p className="text-xs text-muted-foreground">
                {avgResponseTime < 200 ? 'Excellent' : avgResponseTime < 500 ? 'Good' : 'Slow'}
              </p>
            </CardContent>
          </Card>
        </div>

        {/* Endpoint Groups */}
        {Object.entries(groupedEndpoints).map(([category, categoryEndpoints]) => {
          const Icon = getCategoryIcon(category);
          return (
            <Card key={category}>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Icon className="h-5 w-5" />
                  {category}
                  <Badge variant="secondary" className="ml-auto">
                    {categoryEndpoints.filter(ep => ep.status === 'online').length}/{categoryEndpoints.length} Online
                  </Badge>
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Endpoint</TableHead>
                        <TableHead>Method</TableHead>
                        <TableHead>Path</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>Response Time</TableHead>
                        <TableHead>Auth Required</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {categoryEndpoints.map((endpoint, index) => (
                        <TableRow key={index}>
                          <TableCell>
                            <div>
                              <div className="font-medium">{endpoint.name}</div>
                              <div className="text-xs text-gray-500">{endpoint.description}</div>
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant={
                              endpoint.method === 'GET' ? 'default' :
                              endpoint.method === 'POST' ? 'secondary' :
                              endpoint.method === 'PUT' ? 'outline' :
                              'destructive'
                            }>
                              {endpoint.method}
                            </Badge>
                          </TableCell>
                          <TableCell className="font-mono text-xs">{endpoint.path}</TableCell>
                          <TableCell>{getStatusBadge(endpoint.status)}</TableCell>
                          <TableCell>{getResponseTimeBadge(endpoint.responseTime)}</TableCell>
                          <TableCell>
                            {endpoint.requiresAuth ? (
                              <Badge variant="outline"><Shield className="h-3 w-3 mr-1" /> Yes</Badge>
                            ) : (
                              <Badge variant="secondary">No</Badge>
                            )}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </DashboardLayout>
  );
}
