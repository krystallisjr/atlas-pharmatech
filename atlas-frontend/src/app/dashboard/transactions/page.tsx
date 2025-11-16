'use client';

import { useState, useEffect, useMemo } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog';
import { ProtectedRoute } from '@/components/protected-route';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  ShoppingCart,
  CheckCircle2,
  XCircle,
  Clock,
  Package,
  DollarSign,
  Calendar,
  Building2,
  Eye,
  TrendingUp,
  ArrowUpRight,
  ArrowDownLeft,
  Search,
  Filter,
  X,
  Loader2,
  FileText,
  User,
  DownloadCloud,
} from 'lucide-react';
import { MarketplaceService } from '@/lib/services';
import { Transaction, Inquiry } from '@/types/pharmaceutical';
import { toast } from 'react-toastify';
import { format } from 'date-fns';
import { useAuthStore } from '@/lib/auth-store';

interface EnrichedTransaction extends Transaction {
  inquiry?: Inquiry | null;
  buyer_company?: string;
  seller_company?: string;
  product_name?: string;
}

export default function TransactionsPage() {
  const { user } = useAuthStore();
  const [transactions, setTransactions] = useState<EnrichedTransaction[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedTransaction, setSelectedTransaction] = useState<EnrichedTransaction | null>(null);
  const [isDetailsDialogOpen, setIsDetailsDialogOpen] = useState(false);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [dateFilter, setDateFilter] = useState<string>('all');

  useEffect(() => {
    loadTransactions();
  }, []);

  const loadTransactions = async () => {
    try {
      setIsLoading(true);
      const data = await MarketplaceService.getUserTransactions();

      // Enrich transactions with inquiry details
      const enriched = await Promise.all(
        data.map(async (transaction) => {
          try {
            const inquiry = await MarketplaceService.getInquiry(transaction.inquiry_id);
            return {
              ...transaction,
              inquiry,
              product_name: inquiry.inventory?.pharmaceutical?.brand_name || inquiry.inventory?.pharmaceutical?.generic_name || 'Unknown Product',
              buyer_company: inquiry.buyer?.company_name || 'Unknown Buyer',
              seller_company: inquiry.seller?.company_name || 'Unknown Seller',
            };
          } catch {
            return {
              ...transaction,
              inquiry: null,
              product_name: 'Unknown Product',
              buyer_company: 'Unknown Buyer',
              seller_company: 'Unknown Seller',
            };
          }
        })
      );

      setTransactions(enriched);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load transactions');
    } finally {
      setIsLoading(false);
    }
  };

  const handleViewTransaction = (transaction: EnrichedTransaction) => {
    setSelectedTransaction(transaction);
    setIsDetailsDialogOpen(true);
  };

  const handleCompleteTransaction = async (transactionId: string) => {
    if (!confirm('Mark this transaction as completed?')) return;

    try {
      await MarketplaceService.completeTransaction(transactionId, {});
      toast.success('Transaction completed successfully');
      loadTransactions();
      setIsDetailsDialogOpen(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to complete transaction');
    }
  };

  const handleCancelTransaction = async (transactionId: string) => {
    if (!confirm('Are you sure you want to cancel this transaction?')) return;

    try {
      await MarketplaceService.cancelTransaction(transactionId);
      toast.success('Transaction cancelled');
      loadTransactions();
      setIsDetailsDialogOpen(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to cancel transaction');
    }
  };

  const getStatusBadge = (status: string) => {
    const statusConfig = {
      pending: { color: 'bg-yellow-100 text-yellow-800 border-yellow-300', icon: Clock, label: 'Pending' },
      completed: { color: 'bg-green-100 text-green-800 border-green-300', icon: CheckCircle2, label: 'Completed' },
      cancelled: { color: 'bg-red-100 text-red-800 border-red-300', icon: XCircle, label: 'Cancelled' },
    };

    const config = statusConfig[status as keyof typeof statusConfig] || statusConfig.pending;
    const Icon = config.icon;

    return (
      <Badge variant="outline" className={config.color}>
        <Icon className="h-3 w-3 mr-1" />
        {config.label}
      </Badge>
    );
  };

  const isSellerTransaction = (transaction: Transaction) => {
    return transaction.seller_id === user?.id;
  };

  const isBuyerTransaction = (transaction: Transaction) => {
    return transaction.buyer_id === user?.id;
  };

  // Filtered transactions
  const filteredTransactions = useMemo(() => {
    return transactions.filter((t) => {
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        const matchesProduct = t.product_name?.toLowerCase().includes(query);
        const matchesBuyer = t.buyer_company?.toLowerCase().includes(query);
        const matchesSeller = t.seller_company?.toLowerCase().includes(query);
        if (!matchesProduct && !matchesBuyer && !matchesSeller) return false;
      }

      // Status filter
      if (statusFilter !== 'all' && t.status !== statusFilter) return false;

      // Type filter
      if (typeFilter === 'sales' && !isSellerTransaction(t)) return false;
      if (typeFilter === 'purchases' && !isBuyerTransaction(t)) return false;

      // Date filter
      if (dateFilter !== 'all') {
        const transDate = new Date(t.transaction_date);
        const now = new Date();
        const daysDiff = Math.floor((now.getTime() - transDate.getTime()) / (1000 * 60 * 60 * 24));

        if (dateFilter === 'week' && daysDiff > 7) return false;
        if (dateFilter === 'month' && daysDiff > 30) return false;
        if (dateFilter === 'quarter' && daysDiff > 90) return false;
      }

      return true;
    });
  }, [transactions, searchQuery, statusFilter, typeFilter, dateFilter, user]);

  const stats = {
    total: filteredTransactions.length,
    pending: filteredTransactions.filter(t => t.status === 'pending').length,
    completed: filteredTransactions.filter(t => t.status === 'completed').length,
    totalRevenue: filteredTransactions
      .filter(t => t.status === 'completed' && isSellerTransaction(t))
      .reduce((sum, t) => sum + parseFloat(t.total_price || '0'), 0),
    totalSpent: filteredTransactions
      .filter(t => t.status === 'completed' && isBuyerTransaction(t))
      .reduce((sum, t) => sum + parseFloat(t.total_price || '0'), 0),
  };

  const hasActiveFilters = searchQuery || statusFilter !== 'all' || typeFilter !== 'all' || dateFilter !== 'all';

  const clearFilters = () => {
    setSearchQuery('');
    setStatusFilter('all');
    setTypeFilter('all');
    setDateFilter('all');
  };

  if (isLoading) {
    return (
      <ProtectedRoute requireVerification={true}>
        <DashboardLayout>
          <div className="p-6 flex items-center justify-center h-64">
            <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
          </div>
        </DashboardLayout>
      </ProtectedRoute>
    );
  }

  return (
    <ProtectedRoute requireVerification={true}>
      <DashboardLayout>
        <div className="p-6 space-y-6">
          <div>
            <h1 className="text-3xl font-bold text-gray-900">Transactions</h1>
            <p className="text-gray-600">Track your orders and sales</p>
          </div>

          {/* Stats Cards */}
          <div className="grid grid-cols-1 md:grid-cols-5 gap-6">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Total</CardTitle>
                <ShoppingCart className="h-4 w-4 text-blue-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{stats.total}</div>
                <p className="text-xs text-gray-500 mt-1">Transactions</p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Pending</CardTitle>
                <Clock className="h-4 w-4 text-yellow-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-yellow-600">{stats.pending}</div>
                <p className="text-xs text-gray-500 mt-1">Awaiting action</p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Completed</CardTitle>
                <CheckCircle2 className="h-4 w-4 text-green-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-green-600">{stats.completed}</div>
                <p className="text-xs text-gray-500 mt-1">Successfully closed</p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Revenue</CardTitle>
                <ArrowUpRight className="h-4 w-4 text-green-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-green-600">${stats.totalRevenue.toFixed(2)}</div>
                <p className="text-xs text-gray-500 mt-1">From sales</p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Spent</CardTitle>
                <ArrowDownLeft className="h-4 w-4 text-blue-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-blue-600">${stats.totalSpent.toFixed(2)}</div>
                <p className="text-xs text-gray-500 mt-1">From purchases</p>
              </CardContent>
            </Card>
          </div>

          {/* Filters */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Filter className="h-5 w-5" />
                    Filters
                  </CardTitle>
                  <CardDescription>Filter and search transactions</CardDescription>
                </div>
                {hasActiveFilters && (
                  <Button variant="ghost" size="sm" onClick={clearFilters}>
                    <X className="h-4 w-4 mr-1" />
                    Clear All
                  </Button>
                )}
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="search">Search</Label>
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                    <Input
                      id="search"
                      placeholder="Product, buyer, seller..."
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      className="pl-10"
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="status">Status</Label>
                  <Select value={statusFilter} onValueChange={setStatusFilter}>
                    <SelectTrigger id="status">
                      <SelectValue placeholder="All statuses" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All Statuses</SelectItem>
                      <SelectItem value="pending">Pending</SelectItem>
                      <SelectItem value="completed">Completed</SelectItem>
                      <SelectItem value="cancelled">Cancelled</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="type">Type</Label>
                  <Select value={typeFilter} onValueChange={setTypeFilter}>
                    <SelectTrigger id="type">
                      <SelectValue placeholder="All types" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All Types</SelectItem>
                      <SelectItem value="sales">Sales Only</SelectItem>
                      <SelectItem value="purchases">Purchases Only</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="date">Date Range</Label>
                  <Select value={dateFilter} onValueChange={setDateFilter}>
                    <SelectTrigger id="date">
                      <SelectValue placeholder="All time" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All Time</SelectItem>
                      <SelectItem value="week">Past Week</SelectItem>
                      <SelectItem value="month">Past Month</SelectItem>
                      <SelectItem value="quarter">Past Quarter</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Transactions Table */}
          <Card>
            <CardHeader>
              <CardTitle>
                Transactions ({filteredTransactions.length})
                {hasActiveFilters && <span className="text-sm font-normal text-gray-500 ml-2">(filtered from {transactions.length})</span>}
              </CardTitle>
            </CardHeader>
            <CardContent>
              {filteredTransactions.length === 0 ? (
                <div className="text-center py-12">
                  <ShoppingCart className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                  <p className="text-gray-600 mb-2">
                    {hasActiveFilters ? 'No transactions match your filters' : 'No transactions yet'}
                  </p>
                  {hasActiveFilters && (
                    <Button variant="outline" size="sm" onClick={clearFilters}>
                      Clear Filters
                    </Button>
                  )}
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Product</TableHead>
                        <TableHead>Type</TableHead>
                        <TableHead>Company</TableHead>
                        <TableHead>Quantity</TableHead>
                        <TableHead>Unit Price</TableHead>
                        <TableHead>Total</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>Date</TableHead>
                        <TableHead>Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {filteredTransactions.map((transaction) => {
                        const isSeller = isSellerTransaction(transaction);
                        const otherParty = isSeller ? transaction.buyer_company : transaction.seller_company;

                        return (
                          <TableRow key={transaction.id} className="hover:bg-gray-50">
                            <TableCell>
                              <div className="flex items-center gap-2">
                                <Package className="h-4 w-4 text-gray-400" />
                                <span className="font-medium">{transaction.product_name}</span>
                              </div>
                            </TableCell>
                            <TableCell>
                              <Badge variant="outline" className={isSeller ? 'bg-green-50 border-green-300 text-green-700' : 'bg-blue-50 border-blue-300 text-blue-700'}>
                                {isSeller ? (
                                  <><ArrowUpRight className="h-3 w-3 mr-1" /> Sale</>
                                ) : (
                                  <><ArrowDownLeft className="h-3 w-3 mr-1" /> Purchase</>
                                )}
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <div className="flex items-center gap-2">
                                <Building2 className="h-4 w-4 text-gray-400" />
                                <span className="text-sm">{otherParty}</span>
                              </div>
                            </TableCell>
                            <TableCell>{transaction.quantity.toLocaleString()} units</TableCell>
                            <TableCell>${transaction.unit_price}</TableCell>
                            <TableCell className="font-semibold">${transaction.total_price}</TableCell>
                            <TableCell>{getStatusBadge(transaction.status)}</TableCell>
                            <TableCell className="text-sm text-gray-600">
                              {format(new Date(transaction.transaction_date), 'MMM d, yyyy')}
                            </TableCell>
                            <TableCell>
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={() => handleViewTransaction(transaction)}
                              >
                                <Eye className="h-4 w-4 mr-1" />
                                View
                              </Button>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Transaction Details Dialog */}
        <Dialog open={isDetailsDialogOpen} onOpenChange={setIsDetailsDialogOpen}>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Transaction Details</DialogTitle>
              <DialogDescription>
                Complete information about this transaction
              </DialogDescription>
            </DialogHeader>

            {selectedTransaction && (
              <div className="space-y-6">
                {/* Status */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="h-5 w-5 text-gray-400" />
                    <span className="font-medium">Status:</span>
                  </div>
                  {getStatusBadge(selectedTransaction.status)}
                </div>

                {/* Product Info */}
                <div className="border-t pt-4">
                  <h4 className="font-semibold mb-3 flex items-center gap-2">
                    <Package className="h-5 w-5" />
                    Product Information
                  </h4>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <p className="text-gray-600">Product Name</p>
                      <p className="font-medium">{selectedTransaction.product_name}</p>
                    </div>
                    <div>
                      <p className="text-gray-600">Quantity</p>
                      <p className="font-medium">{selectedTransaction.quantity.toLocaleString()} units</p>
                    </div>
                  </div>
                </div>

                {/* Parties */}
                <div className="border-t pt-4">
                  <h4 className="font-semibold mb-3 flex items-center gap-2">
                    <Building2 className="h-5 w-5" />
                    Parties Involved
                  </h4>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <p className="text-gray-600">Seller</p>
                      <p className="font-medium">{selectedTransaction.seller_company}</p>
                    </div>
                    <div>
                      <p className="text-gray-600">Buyer</p>
                      <p className="font-medium">{selectedTransaction.buyer_company}</p>
                    </div>
                  </div>
                </div>

                {/* Financial Details */}
                <div className="border-t pt-4">
                  <h4 className="font-semibold mb-3 flex items-center gap-2">
                    <DollarSign className="h-5 w-5" />
                    Financial Details
                  </h4>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <p className="text-gray-600">Unit Price</p>
                      <p className="font-medium">${selectedTransaction.unit_price}</p>
                    </div>
                    <div>
                      <p className="text-gray-600">Total Amount</p>
                      <p className="font-bold text-lg text-blue-600">${selectedTransaction.total_price}</p>
                    </div>
                  </div>
                </div>

                {/* Date */}
                <div className="border-t pt-4">
                  <div className="flex items-center gap-2 text-sm">
                    <Calendar className="h-4 w-4 text-gray-400" />
                    <span className="text-gray-600">Transaction Date:</span>
                    <span className="font-medium">
                      {format(new Date(selectedTransaction.transaction_date), 'MMMM d, yyyy h:mm a')}
                    </span>
                  </div>
                </div>

                {/* Actions */}
                {selectedTransaction.status === 'pending' && (
                  <DialogFooter className="border-t pt-4">
                    <Button
                      variant="outline"
                      onClick={() => handleCancelTransaction(selectedTransaction.id)}
                    >
                      <XCircle className="h-4 w-4 mr-2" />
                      Cancel Transaction
                    </Button>
                    {isSellerTransaction(selectedTransaction) && (
                      <Button
                        onClick={() => handleCompleteTransaction(selectedTransaction.id)}
                      >
                        <CheckCircle2 className="h-4 w-4 mr-2" />
                        Mark as Completed
                      </Button>
                    )}
                  </DialogFooter>
                )}
              </div>
            )}
          </DialogContent>
        </Dialog>
      </DashboardLayout>
    </ProtectedRoute>
  );
}
