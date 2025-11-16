'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
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
  Plus,
  Search,
  Filter,
  Edit,
  Trash2,
  Package,
  AlertTriangle,
  Calendar,
  DollarSign,
  Download,
  FileText,
  Grid3x3,
  QrCode
} from 'lucide-react';
import { InventoryService, PharmaceuticalService, OpenFdaService } from '@/lib/services';
import type { OpenFdaDrug } from '@/lib/services/openfda-service';
import {
  Inventory,
  Pharmaceutical,
  CreateInventoryRequest,
  UpdateInventoryRequest,
  INVENTORY_STATUS
} from '@/types/pharmaceutical';
import { toast } from 'react-toastify';
import { exportToCSV, exportToExcel, formatInventoryForExport } from '@/lib/utils/export';
import { QRCodeComponent } from '@/components/ui/qr-code';

export default function InventoryPage() {
  const [inventory, setInventory] = useState<Inventory[]>([]);
  const [pharmaceuticals, setPharmaceuticals] = useState<Pharmaceutical[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [isAddDialogOpen, setIsAddDialogOpen] = useState(false);
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<Inventory | null>(null);
  const [formData, setFormData] = useState<CreateInventoryRequest>({
    pharmaceutical_id: '',
    batch_number: '',
    quantity: 0,
    expiry_date: '',
    unit_price: '',
    storage_location: '',
  });

  // OpenFDA Autocomplete state
  const [openfdaQuery, setOpenfdaQuery] = useState('');
  const [openfdaResults, setOpenfdaResults] = useState<OpenFdaDrug[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [showOpenfdaDropdown, setShowOpenfdaDropdown] = useState(false);
  const [selectedDrug, setSelectedDrug] = useState<OpenFdaDrug | null>(null);
  const [useOpenFda, setUseOpenFda] = useState(true); // Toggle between OpenFDA and manual

  useEffect(() => {
    loadData();
  }, []);

  // Debounced OpenFDA search
  useEffect(() => {
    if (!openfdaQuery || openfdaQuery.length < 2) {
      setOpenfdaResults([]);
      setShowOpenfdaDropdown(false);
      return;
    }

    const debounceTimer = setTimeout(async () => {
      try {
        setIsSearching(true);
        const results = await OpenFdaService.search({ query: openfdaQuery, limit: 20 });
        setOpenfdaResults(results);
        setShowOpenfdaDropdown(true);
      } catch (error) {
        console.error('OpenFDA search error:', error);
        setOpenfdaResults([]);
      } finally {
        setIsSearching(false);
      }
    }, 300); // 300ms debounce

    return () => clearTimeout(debounceTimer);
  }, [openfdaQuery]);

  const loadData = async () => {
    try {
      setIsLoading(true);
      const [inventoryData, pharmaData] = await Promise.all([
        InventoryService.getUserInventory(),
        PharmaceuticalService.searchPharmaceuticals({ limit: 1000 })
      ]);
      setInventory(inventoryData);
      setPharmaceuticals(pharmaData);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load inventory');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectOpenFdaDrug = async (drug: OpenFdaDrug) => {
    setSelectedDrug(drug);
    setOpenfdaQuery(`${drug.brand_name} - ${drug.generic_name}`);
    setShowOpenfdaDropdown(false);

    // Create pharmaceutical if it doesn't exist in our system
    try {
      const pharmaData = {
        brand_name: drug.brand_name,
        generic_name: drug.generic_name,
        ndc_code: drug.product_ndc,
        manufacturer: drug.labeler_name,
        dosage_form: drug.dosage_form || '',
        strength: drug.strength || '',
      };

      const createdPharma = await PharmaceuticalService.createPharmaceutical(pharmaData);
      setFormData({ ...formData, pharmaceutical_id: createdPharma.id });
      toast.success('Pharmaceutical from FDA database added to your catalog');
    } catch (error: any) {
      // If it already exists, search for it
      if (error.message?.includes('already exists') || error.message?.includes('Conflict') || error.message?.includes('Resource already exists')) {
        try {
          const existing = await PharmaceuticalService.searchPharmaceuticals({
            ndc_code: drug.product_ndc,
            limit: 1
          });
          if (existing.length > 0) {
            setFormData({ ...formData, pharmaceutical_id: existing[0].id });
            toast.info('Using existing pharmaceutical from your catalog');
          }
        } catch (searchError) {
          toast.error('Failed to find existing pharmaceutical');
        }
      } else {
        toast.error('Failed to add pharmaceutical');
      }
    }
  };

  const handleAddInventory = async () => {
    try {
      // Clean up the data: convert empty strings to null for optional fields
      const requestData: CreateInventoryRequest = {
        ...formData,
        unit_price: formData.unit_price === '' ? null : formData.unit_price,
        storage_location: formData.storage_location === '' ? null : formData.storage_location,
      };

      const newItem = await InventoryService.addInventory(requestData);
      setInventory([newItem, ...inventory]);
      setIsAddDialogOpen(false);
      setFormData({
        pharmaceutical_id: '',
        batch_number: '',
        quantity: 0,
        expiry_date: '',
        unit_price: '',
        storage_location: '',
      });
      setSelectedDrug(null);
      setOpenfdaQuery('');
      toast.success('Inventory item added successfully');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to add inventory');
    }
  };

  const handleUpdateInventory = async () => {
    if (!editingItem) return;

    try {
      const updateData: UpdateInventoryRequest = {
        quantity: formData.quantity,
        unit_price: formData.unit_price,
        storage_location: formData.storage_location,
      };

      const updatedItem = await InventoryService.updateInventory(editingItem.id, updateData);
      setInventory(inventory.map(item =>
        item.id === editingItem.id ? { ...item, ...updatedItem } : item
      ));
      setIsEditDialogOpen(false);
      setEditingItem(null);
      toast.success('Inventory item updated successfully');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to update inventory');
    }
  };

  const handleDeleteInventory = async (id: string) => {
    if (!confirm('Are you sure you want to delete this inventory item?')) return;

    try {
      await InventoryService.deleteInventory(id);
      setInventory(inventory.filter(item => item.id !== id));
      toast.success('Inventory item deleted successfully');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to delete inventory');
    }
  };

  const openEditDialog = (item: Inventory) => {
    setEditingItem(item);
    setFormData({
      pharmaceutical_id: item.pharmaceutical_id,
      batch_number: item.batch_number,
      quantity: item.quantity,
      expiry_date: item.expiry_date,
      unit_price: item.unit_price,
      storage_location: item.storage_location || '',
    });
    setIsEditDialogOpen(true);
  };

  const filteredInventory = inventory.filter(item => {
    const matchesSearch =
      item.pharmaceutical?.brand_name?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.pharmaceutical?.generic_name?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.batch_number.toLowerCase().includes(searchTerm.toLowerCase());

    const matchesStatus = statusFilter === 'all' || item.status === statusFilter;

    return matchesSearch && matchesStatus;
  });

  const getDaysUntilExpiry = (expiryDate: string) => {
    const today = new Date();
    const expiry = new Date(expiryDate);
    const diffTime = expiry.getTime() - today.getTime();
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
    return diffDays;
  };

  const getExpiryBadgeVariant = (days: number) => {
    if (days <= 7) return 'destructive';
    if (days <= 30) return 'secondary';
    return 'outline';
  };

  const handleExportCSV = () => {
    const exportData = formatInventoryForExport(filteredInventory);
    exportToCSV(exportData, {
      filename: `inventory-export-${new Date().toISOString().split('T')[0]}.csv`
    });
  };

  const handleExportExcel = () => {
    const exportData = formatInventoryForExport(filteredInventory);
    exportToExcel(exportData, {
      filename: `inventory-export-${new Date().toISOString().split('T')[0]}.xlsx`,
      sheetName: 'Inventory'
    });
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
            <h1 className="text-3xl font-bold text-gray-900">Inventory Management</h1>
            <p className="text-gray-600">Manage your pharmaceutical stock and inventory</p>
          </div>
          <div className="flex items-center gap-3">
            <Button variant="outline" onClick={handleExportCSV} disabled={filteredInventory.length === 0}>
              <FileText className="h-4 w-4 mr-2" />
              Export CSV
            </Button>
            <Button variant="outline" onClick={handleExportExcel} disabled={filteredInventory.length === 0}>
              <Grid3x3 className="h-4 w-4 mr-2" />
              Export Excel
            </Button>
            <Dialog open={isAddDialogOpen} onOpenChange={setIsAddDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Inventory
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-md">
                <DialogHeader>
                  <DialogTitle>Add New Inventory Item</DialogTitle>
                </DialogHeader>
              <div className="space-y-4">
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <Label htmlFor="pharmaceutical">Pharmaceutical</Label>
                    <button
                      type="button"
                      onClick={() => setUseOpenFda(!useOpenFda)}
                      className="text-xs text-blue-600 hover:text-blue-800"
                    >
                      {useOpenFda ? 'Use Manual Entry' : 'Use FDA Database'}
                    </button>
                  </div>

                  {useOpenFda ? (
                    <div className="relative">
                      <div className="relative">
                        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                        <Input
                          placeholder="Search FDA drug database (e.g., 'Lipitor', 'Aspirin')..."
                          value={openfdaQuery}
                          onChange={(e) => setOpenfdaQuery(e.target.value)}
                          onFocus={() => openfdaQuery && setShowOpenfdaDropdown(true)}
                          className="pl-10"
                        />
                        {isSearching && (
                          <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
                            <div className="animate-spin h-4 w-4 border-2 border-blue-600 border-t-transparent rounded-full" />
                          </div>
                        )}
                      </div>

                      {/* OpenFDA Autocomplete Dropdown */}
                      {showOpenfdaDropdown && openfdaResults.length > 0 && (
                        <div className="absolute z-50 w-full mt-1 bg-white border border-gray-200 rounded-lg shadow-lg max-h-64 overflow-y-auto">
                          {openfdaResults.map((drug) => (
                            <button
                              key={drug.id}
                              type="button"
                              onClick={() => handleSelectOpenFdaDrug(drug)}
                              className="w-full text-left px-4 py-3 hover:bg-blue-50 border-b border-gray-100 last:border-0 transition-colors"
                            >
                              <div className="font-medium text-gray-900">{drug.brand_name}</div>
                              <div className="text-sm text-gray-600">{drug.generic_name}</div>
                              <div className="flex items-center gap-2 mt-1">
                                <span className="text-xs text-gray-500">{drug.labeler_name}</span>
                                {drug.dosage_form && (
                                  <span className="text-xs bg-gray-100 px-2 py-0.5 rounded">{drug.dosage_form}</span>
                                )}
                                {drug.strength && (
                                  <span className="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded">{drug.strength}</span>
                                )}
                              </div>
                            </button>
                          ))}
                        </div>
                      )}

                      {selectedDrug && (
                        <div className="mt-2 p-3 bg-green-50 border border-green-200 rounded-lg">
                          <div className="flex items-start justify-between">
                            <div className="flex-1">
                              <div className="font-medium text-green-900">✓ Selected from FDA Database</div>
                              <div className="text-sm text-green-700 mt-1">
                                {selectedDrug.brand_name} - {selectedDrug.generic_name}
                              </div>
                              <div className="text-xs text-green-600 mt-1">
                                NDC: {selectedDrug.product_ndc} | {selectedDrug.labeler_name}
                              </div>
                            </div>
                            <button
                              type="button"
                              onClick={() => {
                                setSelectedDrug(null);
                                setOpenfdaQuery('');
                                setFormData({ ...formData, pharmaceutical_id: '' });
                              }}
                              className="text-green-600 hover:text-green-800"
                            >
                              ✕
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  ) : (
                    <Select
                      value={formData.pharmaceutical_id}
                      onValueChange={(value) => setFormData({ ...formData, pharmaceutical_id: value })}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select from your catalog" />
                      </SelectTrigger>
                      <SelectContent>
                        {pharmaceuticals.map((pharma) => (
                          <SelectItem key={pharma.id} value={pharma.id}>
                            {pharma.brand_name} - {pharma.generic_name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                </div>
                <div>
                  <Label htmlFor="batch_number">Batch Number</Label>
                  <Input
                    id="batch_number"
                    value={formData.batch_number}
                    onChange={(e) => setFormData({ ...formData, batch_number: e.target.value })}
                    placeholder="Enter batch number"
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label htmlFor="quantity">Quantity</Label>
                    <Input
                      id="quantity"
                      type="number"
                      value={formData.quantity}
                      onChange={(e) => setFormData({ ...formData, quantity: parseInt(e.target.value) || 0 })}
                      placeholder="0"
                    />
                  </div>
                  <div>
                    <Label htmlFor="unit_price">Unit Price ($)</Label>
                    <Input
                      id="unit_price"
                      type="number"
                      step="0.01"
                      value={formData.unit_price}
                      onChange={(e) => setFormData({ ...formData, unit_price: e.target.value })}
                      placeholder="0.00"
                    />
                  </div>
                </div>
                <div>
                  <Label htmlFor="expiry_date">Expiry Date</Label>
                  <Input
                    id="expiry_date"
                    type="date"
                    value={formData.expiry_date}
                    onChange={(e) => setFormData({ ...formData, expiry_date: e.target.value })}
                  />
                </div>
                <div>
                  <Label htmlFor="storage_location">Storage Location</Label>
                  <Input
                    id="storage_location"
                    value={formData.storage_location}
                    onChange={(e) => setFormData({ ...formData, storage_location: e.target.value })}
                    placeholder="Enter storage location"
                  />
                </div>
                <Button onClick={handleAddInventory} className="w-full">
                  Add Inventory Item
                </Button>
              </div>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        {/* Enhanced Search Section */}
        <div className="bg-gradient-to-r from-green-50 to-emerald-50 rounded-xl p-6 border border-green-100">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-xl font-semibold text-gray-900">Inventory Search</h2>
              <p className="text-sm text-gray-600">Search your pharmaceutical inventory by name, batch, or location</p>
            </div>
            <Search className="h-8 w-8 text-green-600" />
          </div>
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="md:col-span-2">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  placeholder="Search by product name, batch number, or location..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="pl-10 bg-white border-gray-200 focus:ring-2 focus:ring-green-500 focus:border-green-500"
                />
              </div>
            </div>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger className="bg-white border-gray-200 focus:ring-2 focus:ring-green-500 focus:border-green-500">
                <SelectValue placeholder="Filter by status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Status</SelectItem>
                <SelectItem value={INVENTORY_STATUS.AVAILABLE}>Available</SelectItem>
                <SelectItem value={INVENTORY_STATUS.RESERVED}>Reserved</SelectItem>
                <SelectItem value={INVENTORY_STATUS.SOLD}>Sold</SelectItem>
              </SelectContent>
            </Select>
            <Button className="bg-green-600 hover:bg-green-700">
              <Filter className="h-4 w-4 mr-2" />
              Apply Filters
            </Button>
          </div>
        </div>

        {/* Export Summary */}
        {filteredInventory.length > 0 && (
          <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-xl p-4 border border-blue-100">
            <div className="flex items-center justify-between flex-wrap gap-3">
              <div className="flex items-center gap-3">
                <Download className="h-5 w-5 text-blue-600" />
                <div>
                  <p className="text-sm font-medium text-gray-900">
                    Export {filteredInventory.length} {filteredInventory.length === 1 ? 'item' : 'items'}
                  </p>
                  <p className="text-xs text-gray-600">
                    Total value: ${filteredInventory.reduce((sum, item) => sum + (parseFloat(item.unit_price) * item.quantity), 0).toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" onClick={handleExportCSV}>
                  <FileText className="h-3 w-3 mr-1" />
                  CSV
                </Button>
                <Button variant="outline" size="sm" onClick={handleExportExcel}>
                  <Grid3x3 className="h-3 w-3 mr-1" />
                  Excel
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* Inventory Table */}
        <Card>
          <CardHeader>
            <CardTitle>Inventory Items ({filteredInventory.length})</CardTitle>
          </CardHeader>
          <CardContent>
            {filteredInventory.length === 0 ? (
              <div className="text-center py-8">
                <Package className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <h3 className="text-lg font-medium text-gray-900 mb-2">No inventory items found</h3>
                <p className="text-gray-600 mb-4">
                  {searchTerm || statusFilter !== 'all'
                    ? 'Try adjusting your filters'
                    : 'Get started by adding your first inventory item'
                  }
                </p>
                {!searchTerm && statusFilter === 'all' && (
                  <Button onClick={() => setIsAddDialogOpen(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    Add First Item
                  </Button>
                )}
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Product</TableHead>
                      <TableHead>Batch Number</TableHead>
                      <TableHead>Quantity</TableHead>
                      <TableHead>Unit Price</TableHead>
                      <TableHead>Expiry Date</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredInventory.map((item) => {
                      const daysUntilExpiry = getDaysUntilExpiry(item.expiry_date);
                      return (
                        <TableRow key={item.id}>
                          <TableCell>
                            <div>
                              <div className="font-medium text-gray-900">
                                {item.pharmaceutical?.brand_name}
                              </div>
                              <div className="text-sm text-gray-500">
                                {item.pharmaceutical?.generic_name}
                              </div>
                              <div className="text-xs text-gray-400">
                                {item.pharmaceutical?.manufacturer}
                              </div>
                            </div>
                          </TableCell>
                          <TableCell className="font-mono text-sm">
                            {item.batch_number}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center">
                              <span className={item.quantity < 10 ? 'text-red-600 font-medium' : ''}>
                                {item.quantity}
                              </span>
                              {item.quantity < 10 && (
                                <AlertTriangle className="h-3 w-3 text-red-600 ml-1" />
                              )}
                            </div>
                          </TableCell>
                          <TableCell>${parseFloat(item.unit_price).toFixed(2)}</TableCell>
                          <TableCell>
                            <div className="flex items-center space-x-2">
                              <span className="text-sm">{item.expiry_date}</span>
                              <Badge variant={getExpiryBadgeVariant(daysUntilExpiry)}>
                                {daysUntilExpiry > 0 ? `${daysUntilExpiry} days` : 'Expired'}
                              </Badge>
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge
                              variant={
                                item.status === INVENTORY_STATUS.AVAILABLE ? 'default' :
                                item.status === INVENTORY_STATUS.RESERVED ? 'secondary' :
                                item.status === INVENTORY_STATUS.SOLD ? 'outline' : 'destructive'
                              }
                            >
                              {item.status}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center space-x-2">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => openEditDialog(item)}
                              >
                                <Edit className="h-4 w-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => handleDeleteInventory(item.id)}
                                className="text-red-600 hover:text-red-700"
                              >
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            </div>
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

        {/* Edit Dialog */}
        <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Edit Inventory Item</DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="edit_quantity">Quantity</Label>
                  <Input
                    id="edit_quantity"
                    type="number"
                    value={formData.quantity}
                    onChange={(e) => setFormData({ ...formData, quantity: parseInt(e.target.value) || 0 })}
                  />
                </div>
                <div>
                  <Label htmlFor="edit_unit_price">Unit Price ($)</Label>
                  <Input
                    id="edit_unit_price"
                    type="number"
                    step="0.01"
                    value={formData.unit_price}
                    onChange={(e) => setFormData({ ...formData, unit_price: e.target.value })}
                  />
                </div>
              </div>
              <div>
                <Label htmlFor="edit_storage_location">Storage Location</Label>
                <Input
                  id="edit_storage_location"
                  value={formData.storage_location}
                  onChange={(e) => setFormData({ ...formData, storage_location: e.target.value })}
                />
              </div>
              <div className="flex space-x-3">
                <Button onClick={handleUpdateInventory} className="flex-1">
                  Update Item
                </Button>
                <Button
                  variant="outline"
                  onClick={() => setIsEditDialogOpen(false)}
                  className="flex-1"
                >
                  Cancel
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </div>
    </DashboardLayout>
  );
}