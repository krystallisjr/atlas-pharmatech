'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { ProtectedRoute } from '@/components/protected-route';
import { InquiryChat } from '@/components/inquiry-chat';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  MessageSquare,
  Clock,
  CheckCircle,
  XCircle,
  User,
  Package,
  Calendar,
  DollarSign,
  Eye,
  Building2,
  ArrowRight,
  AlertTriangle,
  ShoppingCart
} from 'lucide-react';
import { MarketplaceService } from '@/lib/services';
import { Inquiry } from '@/types/pharmaceutical';
import { toast } from 'react-toastify';
import { format } from 'date-fns';
import { useAuthStore } from '@/lib/auth-store';

export default function InquiriesPage() {
  const { user } = useAuthStore();
  const [buyerInquiries, setBuyerInquiries] = useState<Inquiry[]>([]);
  const [sellerInquiries, setSellerInquiries] = useState<Inquiry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedInquiry, setSelectedInquiry] = useState<Inquiry | null>(null);
  const [isDetailsDialogOpen, setIsDetailsDialogOpen] = useState(false);
  const [isCreatingTransaction, setIsCreatingTransaction] = useState(false);

  useEffect(() => {
    loadInquiries();
  }, []);

  const loadInquiries = async () => {
    try {
      setIsLoading(true);
      const [buyerData, sellerData] = await Promise.all([
        MarketplaceService.getBuyerInquiries(),
        MarketplaceService.getSellerInquiries(),
      ]);
      setBuyerInquiries(buyerData);
      setSellerInquiries(sellerData);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load inquiries');
    } finally {
      setIsLoading(false);
    }
  };

  const handleViewInquiry = (inquiry: Inquiry) => {
    console.log('Selected inquiry:', inquiry);
    setSelectedInquiry(inquiry);
    setIsDetailsDialogOpen(true);
  };

  const handleUpdateStatus = async (inquiryId: string, status: 'accepted' | 'rejected') => {
    try {
      await MarketplaceService.updateInquiryStatus(inquiryId, { status });
      toast.success(`Inquiry ${status}`);
      loadInquiries();
      setIsDetailsDialogOpen(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to update inquiry');
    }
  };

  const handleCreateTransaction = async (inquiry: Inquiry) => {
    console.log('Creating transaction for inquiry:', inquiry);

    if (!inquiry.inventory) {
      toast.error('Inventory information missing. Please refresh and try again.');
      console.error('Inventory missing:', inquiry);
      return;
    }

    if (!inquiry.inventory.unit_price) {
      toast.error('Product price information missing');
      return;
    }

    try {
      setIsCreatingTransaction(true);

      const transaction = await MarketplaceService.createTransaction({
        inquiry_id: inquiry.id,
        quantity: inquiry.quantity_requested,
        unit_price: inquiry.inventory.unit_price,
      });

      // Update inquiry status to converted
      await MarketplaceService.updateInquiryStatus(inquiry.id, { status: 'accepted' });

      toast.success('Transaction created successfully!');
      loadInquiries();
      setIsDetailsDialogOpen(false);
    } catch (error) {
      console.error('Transaction creation error:', error);
      toast.error(error instanceof Error ? error.message : 'Failed to create transaction');
    } finally {
      setIsCreatingTransaction(false);
    }
  };

  const getStatusBadge = (status: string) => {
    const statusConfig = {
      pending: { color: 'bg-yellow-100 text-yellow-800', icon: Clock, label: 'Pending' },
      negotiating: { color: 'bg-blue-100 text-blue-800', icon: MessageSquare, label: 'Negotiating' },
      accepted: { color: 'bg-green-100 text-green-800', icon: CheckCircle, label: 'Accepted' },
      rejected: { color: 'bg-red-100 text-red-800', icon: XCircle, label: 'Rejected' },
      converted_to_transaction: { color: 'bg-purple-100 text-purple-800', icon: ShoppingCart, label: 'Transaction' },
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

  const renderInquiriesTable = (inquiries: Inquiry[], isSeller: boolean) => {
    if (inquiries.length === 0) {
      return (
        <div className="text-center py-12">
          <MessageSquare className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <p className="text-gray-600">No inquiries yet</p>
        </div>
      );
    }

    return (
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Product</TableHead>
            <TableHead>{isSeller ? 'Buyer' : 'Seller'}</TableHead>
            <TableHead>Quantity</TableHead>
            <TableHead>Price</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Date</TableHead>
            <TableHead>Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {inquiries.map((inquiry) => (
            <TableRow key={inquiry.id}>
              <TableCell>
                <div className="flex items-center space-x-3">
                  <Package className="h-5 w-5 text-gray-400" />
                  <div>
                    <div className="font-medium">{inquiry.inventory?.pharmaceutical?.brand_name || 'N/A'}</div>
                    <div className="text-sm text-gray-500">
                      {inquiry.inventory?.pharmaceutical?.generic_name || ''}
                    </div>
                  </div>
                </div>
              </TableCell>
              <TableCell>
                <div className="flex items-center space-x-2">
                  <Building2 className="h-4 w-4 text-gray-400" />
                  <span className="text-sm">
                    {isSeller ? inquiry.buyer?.company_name : inquiry.seller?.company_name}
                  </span>
                </div>
              </TableCell>
              <TableCell>{inquiry.quantity_requested.toLocaleString()} units</TableCell>
              <TableCell>${inquiry.inventory?.unit_price || '0'}</TableCell>
              <TableCell>{getStatusBadge(inquiry.status)}</TableCell>
              <TableCell className="text-sm text-gray-600">
                {format(new Date(inquiry.created_at), 'MMM d, yyyy')}
              </TableCell>
              <TableCell>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleViewInquiry(inquiry)}
                >
                  <Eye className="h-4 w-4 mr-1" />
                  View
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    );
  };

  const calculateTotal = (inquiry: Inquiry) => {
    if (!inquiry.inventory?.unit_price) return '0';
    const price = parseFloat(inquiry.inventory.unit_price);
    return (price * inquiry.quantity_requested).toFixed(2);
  };

  const isSellerInquiry = (inquiry: Inquiry) => {
    return inquiry.inventory?.user_id === user?.id;
  };

  return (
    <ProtectedRoute requireVerification={true}>
      <DashboardLayout>
        <div className="p-6 space-y-6">
          {/* Header */}
          <div>
            <h1 className="text-3xl font-bold text-gray-900">Inquiries</h1>
            <p className="text-gray-600">Manage purchase requests and negotiations</p>
          </div>

          {/* Stats Cards */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Buyer Inquiries</CardTitle>
                <MessageSquare className="h-4 w-4 text-blue-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{buyerInquiries.length}</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Seller Inquiries</CardTitle>
                <Package className="h-4 w-4 text-green-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{sellerInquiries.length}</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Pending</CardTitle>
                <Clock className="h-4 w-4 text-yellow-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {[...buyerInquiries, ...sellerInquiries].filter(i => i.status === 'pending').length}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-gray-600">Negotiating</CardTitle>
                <MessageSquare className="h-4 w-4 text-blue-600" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {[...buyerInquiries, ...sellerInquiries].filter(i => i.status === 'negotiating').length}
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Inquiries Tabs */}
          <Tabs defaultValue="buyer" className="space-y-4">
            <TabsList>
              <TabsTrigger value="buyer">
                <User className="h-4 w-4 mr-2" />
                My Purchase Requests ({buyerInquiries.length})
              </TabsTrigger>
              <TabsTrigger value="seller">
                <Package className="h-4 w-4 mr-2" />
                Received Inquiries ({sellerInquiries.length})
              </TabsTrigger>
            </TabsList>

            <TabsContent value="buyer">
              <Card>
                <CardHeader>
                  <CardTitle>Purchase Requests</CardTitle>
                  <p className="text-sm text-gray-600">Inquiries you've sent to sellers</p>
                </CardHeader>
                <CardContent>
                  {isLoading ? (
                    <div className="text-center py-8">Loading...</div>
                  ) : (
                    renderInquiriesTable(buyerInquiries, false)
                  )}
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="seller">
              <Card>
                <CardHeader>
                  <CardTitle>Received Inquiries</CardTitle>
                  <p className="text-sm text-gray-600">Purchase requests from buyers</p>
                </CardHeader>
                <CardContent>
                  {isLoading ? (
                    <div className="text-center py-8">Loading...</div>
                  ) : (
                    renderInquiriesTable(sellerInquiries, true)
                  )}
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>

          {/* Inquiry Details Dialog */}
          <Dialog open={isDetailsDialogOpen} onOpenChange={setIsDetailsDialogOpen}>
            <DialogContent className="max-w-4xl max-h-[90vh] overflow-y-auto">
              <DialogHeader>
                <DialogTitle>Inquiry Details</DialogTitle>
                <DialogDescription>
                  Negotiate and finalize the transaction
                </DialogDescription>
              </DialogHeader>

              {selectedInquiry && selectedInquiry.id && (
                <div className="space-y-6">
                  {/* Debug Info */}
                  <div className="text-xs text-gray-400">
                    Inquiry ID: {selectedInquiry.id} | Has Inventory: {selectedInquiry.inventory ? 'Yes' : 'No'}
                  </div>
                  {/* Status Alert */}
                  {selectedInquiry.status === 'converted_to_transaction' && (
                    <Alert>
                      <CheckCircle className="h-4 w-4" />
                      <AlertDescription>
                        This inquiry has been converted to a transaction. Check the Transactions page for details.
                      </AlertDescription>
                    </Alert>
                  )}

                  {/* Product & Party Details */}
                  <div className="grid grid-cols-2 gap-6">
                    <Card>
                      <CardHeader>
                        <CardTitle className="text-lg">Product Details</CardTitle>
                      </CardHeader>
                      <CardContent className="space-y-3">
                        <div>
                          <div className="text-sm text-gray-600">Product</div>
                          <div className="font-medium">{selectedInquiry.inventory?.pharmaceutical?.brand_name}</div>
                          <div className="text-sm text-gray-500">{selectedInquiry.inventory?.pharmaceutical?.generic_name}</div>
                        </div>
                        <div>
                          <div className="text-sm text-gray-600">Quantity Requested</div>
                          <div className="font-medium">{selectedInquiry.quantity_requested.toLocaleString()} units</div>
                        </div>
                        <div>
                          <div className="text-sm text-gray-600">Unit Price</div>
                          <div className="font-medium">${selectedInquiry.inventory?.unit_price}</div>
                        </div>
                        <div>
                          <div className="text-sm text-gray-600">Total Price</div>
                          <div className="font-medium text-lg text-green-600">${calculateTotal(selectedInquiry)}</div>
                        </div>
                        <div>
                          <div className="text-sm text-gray-600">Batch Number</div>
                          <div className="font-medium">{selectedInquiry.inventory?.batch_number}</div>
                        </div>
                        <div>
                          <div className="text-sm text-gray-600">Expiry Date</div>
                          <div className="font-medium">
                            {selectedInquiry.inventory?.expiry_date
                              ? format(new Date(selectedInquiry.inventory.expiry_date), 'MMM d, yyyy')
                              : 'N/A'
                            }
                          </div>
                        </div>
                      </CardContent>
                    </Card>

                    <Card>
                      <CardHeader>
                        <CardTitle className="text-lg">Parties Involved</CardTitle>
                      </CardHeader>
                      <CardContent className="space-y-4">
                        <div>
                          <div className="text-sm text-gray-600 mb-2">Buyer</div>
                          <div className="flex items-center space-x-2">
                            <Building2 className="h-5 w-5 text-blue-600" />
                            <div>
                              <div className="font-medium">{selectedInquiry.buyer?.company_name}</div>
                              <div className="text-sm text-gray-500">{selectedInquiry.buyer?.email}</div>
                              {selectedInquiry.buyer?.contact_person && (
                                <div className="text-sm text-gray-500">Contact: {selectedInquiry.buyer.contact_person}</div>
                              )}
                            </div>
                          </div>
                        </div>

                        <div className="border-t pt-4">
                          <div className="text-sm text-gray-600 mb-2">Seller</div>
                          <div className="flex items-center space-x-2">
                            <Building2 className="h-5 w-5 text-green-600" />
                            <div>
                              <div className="font-medium">{selectedInquiry.seller?.company_name}</div>
                              <div className="text-sm text-gray-500">{selectedInquiry.seller?.email}</div>
                              {selectedInquiry.seller?.contact_person && (
                                <div className="text-sm text-gray-500">Contact: {selectedInquiry.seller.contact_person}</div>
                              )}
                            </div>
                          </div>
                        </div>

                        <div className="border-t pt-4">
                          <div className="text-sm text-gray-600 mb-2">Status</div>
                          {getStatusBadge(selectedInquiry.status)}
                        </div>

                        <div>
                          <div className="text-sm text-gray-600 mb-2">Created</div>
                          <div className="text-sm">{format(new Date(selectedInquiry.created_at), 'MMM d, yyyy h:mm a')}</div>
                        </div>
                      </CardContent>
                    </Card>
                  </div>

                  {/* Chat Component */}
                  {selectedInquiry.status !== 'converted_to_transaction' &&
                   selectedInquiry.status !== 'rejected' && (
                    <InquiryChat
                      inquiryId={selectedInquiry.id}
                      buyerCompany={selectedInquiry.buyer?.company_name || 'Buyer'}
                      sellerCompany={selectedInquiry.seller?.company_name || 'Seller'}
                      isSeller={isSellerInquiry(selectedInquiry)}
                      onMessageSent={loadInquiries}
                    />
                  )}

                  {/* Actions for Seller */}
                  {isSellerInquiry(selectedInquiry) &&
                   selectedInquiry.status !== 'converted_to_transaction' &&
                   selectedInquiry.status !== 'rejected' && (
                    <Card className="bg-gray-50">
                      <CardHeader>
                        <CardTitle className="text-lg">Seller Actions</CardTitle>
                      </CardHeader>
                      <CardContent>
                        <div className="flex gap-3">
                          {selectedInquiry.status === 'pending' && (
                            <Button
                              onClick={() => handleUpdateStatus(selectedInquiry.id, 'rejected')}
                              variant="outline"
                              className="border-red-300 text-red-700 hover:bg-red-50"
                            >
                              <XCircle className="h-4 w-4 mr-2" />
                              Reject Inquiry
                            </Button>
                          )}

                          <Button
                            onClick={() => {
                              console.log('Button clicked, selectedInquiry:', selectedInquiry);
                              if (selectedInquiry) {
                                handleCreateTransaction(selectedInquiry);
                              } else {
                                toast.error('No inquiry selected');
                              }
                            }}
                            disabled={isCreatingTransaction || !selectedInquiry}
                            className="bg-green-600 hover:bg-green-700"
                          >
                            {isCreatingTransaction ? (
                              'Creating...'
                            ) : (
                              <>
                                <ShoppingCart className="h-4 w-4 mr-2" />
                                Create Transaction
                              </>
                            )}
                          </Button>
                        </div>
                        <p className="text-sm text-gray-600 mt-3">
                          Once both parties agree via chat, create a transaction to finalize the order.
                        </p>
                      </CardContent>
                    </Card>
                  )}
                </div>
              )}
            </DialogContent>
          </Dialog>
        </div>
      </DashboardLayout>
    </ProtectedRoute>
  );
}
