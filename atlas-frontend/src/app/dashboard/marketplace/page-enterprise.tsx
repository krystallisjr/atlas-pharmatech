'use client';

import { useState, useEffect } from 'react';
import { useAuth } from '@/contexts/auth-context';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Search,
  Filter,
  MessageSquare,
  Calendar,
  DollarSign,
  Package,
  MapPin,
  Building,
  Clock,
  AlertTriangle,
  TrendingUp,
  BarChart3,
  Grid3x3,
  List,
  ChevronDown,
  ChevronUp,
  SlidersHorizontal
} from 'lucide-react';
import { InventoryService, MarketplaceService, PharmaceuticalService } from '@/lib/services';
import {
  Inventory,
  CreateInquiryRequest,
  Manufacturer,
  Category,
} from '@/types/pharmaceutical';
import { toast } from 'react-toastify';

type ViewMode = 'grid' | 'list';
type SortBy = 'price_asc' | 'price_desc' | 'quantity_asc' | 'quantity_desc' | 'expiry_asc' | 'expiry_desc' | 'name_asc' | 'name_desc';

// Product type categories from FDA data
const PRODUCT_TYPES = [
  'HUMAN OTC DRUG',
  'HUMAN PRESCRIPTION DRUG',
  'BULK INGREDIENT',
  'DRUG FOR FURTHER PROCESSING'
];

// Common dosage forms from market analysis
const DOSAGE_FORMS = [
  'TABLET',
  'CAPSULE',
  'LIQUID',
  'POWDER',
  'INJECTION',
  'CREAM',
  'GEL',
  'SOLUTION',
  'OINTMENT',
  'PATCH'
];

export default function EnterpriseMarketplacePage() {
  const { user } = useAuth();

  // Data state
  const [marketplaceInventory, setMarketplaceInventory] = useState<Inventory[]>([]);
  const [filteredInventory, setFilteredInventory] = useState<Inventory[]>([]);
  const [manufacturers, setManufacturers] = useState<Manufacturer[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // UI state
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [sortBy, setSortBy] = useState<SortBy>('expiry_asc');
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 25;

  // Filter state
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedManufacturers, setSelectedManufacturers] = useState<string[]>([]);
  const [selectedProductTypes, setSelectedProductTypes] = useState<string[]>([]);
  const [selectedDosageForms, setSelectedDosageForms] = useState<string[]>([]);
  const [priceRange, setPriceRange] = useState({ min: '', max: '' });
  const [quantityRange, setQuantityRange] = useState({ min: '', max: '' });
  const [expiryDays, setExpiryDays] = useState<string>('all');

  // Dialog state
  const [isInquiryDialogOpen, setIsInquiryDialogOpen] = useState(false);
  const [selectedItem, setSelectedItem] = useState<Inventory | null>(null);
  const [inquiryForm, setInquiryForm] = useState<CreateInquiryRequest>({
    inventory_id: '',
    quantity_requested: 0,
    message: '',
  });

  // Sidebar collapse state
  const [collapsedSections, setCollapsedSections] = useState({
    productType: false,
    dosageForm: false,
    manufacturer: false,
    price: false,
    expiry: false
  });

  useEffect(() => {
    loadMarketplaceData();
  }, []);

  useEffect(() => {
    applyFiltersAndSorting();
  }, [
    marketplaceInventory,
    searchTerm,
    selectedManufacturers,
    selectedProductTypes,
    selectedDosageForms,
    priceRange,
    quantityRange,
    expiryDays,
    sortBy
  ]);

  const loadMarketplaceData = async () => {
    try {
      setIsLoading(true);
      const [inventoryData, manufacturersData, categoriesData] = await Promise.all([
        InventoryService.searchMarketplaceInventory({ available_only: true, limit: 1000 }),
        PharmaceuticalService.getManufacturers(),
        PharmaceuticalService.getCategories(),
      ]);

      setMarketplaceInventory(inventoryData);
      setManufacturers(manufacturersData);
      setCategories(categoriesData);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load marketplace data');
    } finally {
      setIsLoading(false);
    }
  };

  const applyFiltersAndSorting = () => {
    let filtered = [...marketplaceInventory];

    // Text search
    if (searchTerm) {
      const search = searchTerm.toLowerCase();
      filtered = filtered.filter(item =>
        item.pharmaceutical?.brand_name?.toLowerCase().includes(search) ||
        item.pharmaceutical?.generic_name?.toLowerCase().includes(search) ||
        item.pharmaceutical?.manufacturer?.toLowerCase().includes(search) ||
        item.pharmaceutical?.ndc_code?.toLowerCase().includes(search)
      );
    }

    // Manufacturer filter
    if (selectedManufacturers.length > 0) {
      filtered = filtered.filter(item =>
        selectedManufacturers.includes(item.pharmaceutical?.manufacturer || '')
      );
    }

    // Dosage form filter
    if (selectedDosageForms.length > 0) {
      filtered = filtered.filter(item =>
        selectedDosageForms.some(form =>
          item.pharmaceutical?.dosage_form?.toUpperCase().includes(form)
        )
      );
    }

    // Price filter
    if (priceRange.min) {
      filtered = filtered.filter(item =>
        item.unit_price && parseFloat(item.unit_price) >= parseFloat(priceRange.min)
      );
    }
    if (priceRange.max) {
      filtered = filtered.filter(item =>
        item.unit_price && parseFloat(item.unit_price) <= parseFloat(priceRange.max)
      );
    }

    // Quantity filter
    if (quantityRange.min) {
      filtered = filtered.filter(item =>
        item.quantity >= parseInt(quantityRange.min)
      );
    }
    if (quantityRange.max) {
      filtered = filtered.filter(item =>
        item.quantity <= parseInt(quantityRange.max)
      );
    }

    // Expiry filter
    if (expiryDays !== 'all') {
      const daysThreshold = parseInt(expiryDays);
      filtered = filtered.filter(item => {
        const days = getDaysUntilExpiry(item.expiry_date);
        return days <= daysThreshold;
      });
    }

    // Sorting
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'price_asc':
          return (parseFloat(a.unit_price || '0') - parseFloat(b.unit_price || '0'));
        case 'price_desc':
          return (parseFloat(b.unit_price || '0') - parseFloat(a.unit_price || '0'));
        case 'quantity_asc':
          return a.quantity - b.quantity;
        case 'quantity_desc':
          return b.quantity - a.quantity;
        case 'expiry_asc':
          return new Date(a.expiry_date).getTime() - new Date(b.expiry_date).getTime();
        case 'expiry_desc':
          return new Date(b.expiry_date).getTime() - new Date(a.expiry_date).getTime();
        case 'name_asc':
          return (a.pharmaceutical?.brand_name || '').localeCompare(b.pharmaceutical?.brand_name || '');
        case 'name_desc':
          return (b.pharmaceutical?.brand_name || '').localeCompare(a.pharmaceutical?.brand_name || '');
        default:
          return 0;
      }
    });

    setFilteredInventory(filtered);
    setCurrentPage(1);
  };

  const toggleManufacturer = (manufacturer: string) => {
    setSelectedManufacturers(prev =>
      prev.includes(manufacturer)
        ? prev.filter(m => m !== manufacturer)
        : [...prev, manufacturer]
    );
  };

  const toggleDosageForm = (form: string) => {
    setSelectedDosageForms(prev =>
      prev.includes(form)
        ? prev.filter(f => f !== form)
        : [...prev, form]
    );
  };

  const clearAllFilters = () => {
    setSearchTerm('');
    setSelectedManufacturers([]);
    setSelectedProductTypes([]);
    setSelectedDosageForms([]);
    setPriceRange({ min: '', max: '' });
    setQuantityRange({ min: '', max: '' });
    setExpiryDays('all');
  };

  const getDaysUntilExpiry = (expiryDate: string) => {
    const today = new Date();
    const expiry = new Date(expiryDate);
    const diffTime = expiry.getTime() - today.getTime();
    return Math.ceil(diffTime / (1000 * 60 * 60 * 24));
  };

  const getExpiryStatus = (days: number) => {
    if (days <= 7) return { label: 'Critical', variant: 'destructive' as const };
    if (days <= 30) return { label: 'Warning', variant: 'secondary' as const };
    if (days <= 90) return { label: 'Moderate', variant: 'outline' as const };
    return { label: 'Good', variant: 'outline' as const };
  };

  const handleCreateInquiry = async () => {
    if (!selectedItem) return;

    try {
      const inquiryData = {
        ...inquiryForm,
        message: inquiryForm.message?.trim() || undefined,
      };

      await MarketplaceService.createInquiry(inquiryData);
      setIsInquiryDialogOpen(false);
      setSelectedItem(null);
      setInquiryForm({ inventory_id: '', quantity_requested: 0, message: '' });
      toast.success('Inquiry sent successfully');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to send inquiry');
    }
  };

  const openInquiryDialog = (item: Inventory) => {
    setSelectedItem(item);
    setInquiryForm({
      inventory_id: item.id,
      quantity_requested: Math.min(item.quantity, 1),
      message: '',
    });
    setIsInquiryDialogOpen(true);
  };

  // Calculate stats
  const stats = {
    totalProducts: filteredInventory.length,
    totalQuantity: filteredInventory.reduce((sum, item) => sum + item.quantity, 0),
    averagePrice: filteredInventory.length > 0
      ? (filteredInventory.reduce((sum, item) => sum + parseFloat(item.unit_price || '0'), 0) / filteredInventory.length).toFixed(2)
      : '0.00',
    expiringSoon: filteredInventory.filter(item => getDaysUntilExpiry(item.expiry_date) <= 30).length
  };

  // Pagination
  const paginatedInventory = filteredInventory.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  );
  const totalPages = Math.ceil(filteredInventory.length / itemsPerPage);

  if (isLoading) {
    return (
      <DashboardLayout>
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-blue-600 mx-auto mb-4"></div>
            <p className="text-gray-600 text-lg">Loading marketplace data...</p>
          </div>
        </div>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      <div className="flex h-full bg-gray-50">
        {/* Advanced Filter Sidebar */}
        <div className="w-80 bg-white border-r border-gray-200 overflow-y-auto">
          <div className="p-6">
            <div className="flex items-center justify-between mb-6">
              <div>
                <h2 className="text-lg font-semibold text-gray-900">Filters</h2>
                <p className="text-sm text-gray-500">Refine your search</p>
              </div>
              <Button variant="ghost" size="sm" onClick={clearAllFilters}>
                Clear All
              </Button>
            </div>

            {/* Active Filters Count */}
            {(selectedManufacturers.length + selectedDosageForms.length + selectedProductTypes.length) > 0 && (
              <div className="mb-4 p-3 bg-blue-50 rounded-lg">
                <p className="text-sm font-medium text-blue-900">
                  {selectedManufacturers.length + selectedDosageForms.length + selectedProductTypes.length} filters active
                </p>
              </div>
            )}

            {/* Dosage Form Filter */}
            <div className="mb-6">
              <button
                onClick={() => setCollapsedSections(prev => ({ ...prev, dosageForm: !prev.dosageForm }))}
                className="flex items-center justify-between w-full mb-3"
              >
                <Label className="text-sm font-semibold text-gray-700">Dosage Form</Label>
                {collapsedSections.dosageForm ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
              </button>
              {!collapsedSections.dosageForm && (
                <div className="space-y-2 ml-1">
                  {DOSAGE_FORMS.map(form => (
                    <div key={form} className="flex items-center">
                      <Checkbox
                        id={`dosage-${form}`}
                        checked={selectedDosageForms.includes(form)}
                        onCheckedChange={() => toggleDosageForm(form)}
                      />
                      <label
                        htmlFor={`dosage-${form}`}
                        className="ml-2 text-sm text-gray-700 cursor-pointer"
                      >
                        {form.charAt(0) + form.slice(1).toLowerCase()}
                      </label>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <Separator className="my-4" />

            {/* Manufacturer Filter */}
            <div className="mb-6">
              <button
                onClick={() => setCollapsedSections(prev => ({ ...prev, manufacturer: !prev.manufacturer }))}
                className="flex items-center justify-between w-full mb-3"
              >
                <Label className="text-sm font-semibold text-gray-700">Manufacturer</Label>
                {collapsedSections.manufacturer ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
              </button>
              {!collapsedSections.manufacturer && (
                <div className="space-y-2 ml-1 max-h-64 overflow-y-auto">
                  {manufacturers.slice(0, 15).map(mfg => (
                    <div key={mfg.manufacturer} className="flex items-center">
                      <Checkbox
                        id={`mfg-${mfg.manufacturer}`}
                        checked={selectedManufacturers.includes(mfg.manufacturer)}
                        onCheckedChange={() => toggleManufacturer(mfg.manufacturer)}
                      />
                      <label
                        htmlFor={`mfg-${mfg.manufacturer}`}
                        className="ml-2 text-sm text-gray-700 cursor-pointer flex-1"
                      >
                        {mfg.manufacturer}
                        <span className="text-gray-400 ml-1">({mfg.count})</span>
                      </label>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <Separator className="my-4" />

            {/* Price Range Filter */}
            <div className="mb-6">
              <button
                onClick={() => setCollapsedSections(prev => ({ ...prev, price: !prev.price }))}
                className="flex items-center justify-between w-full mb-3"
              >
                <Label className="text-sm font-semibold text-gray-700">Price Range</Label>
                {collapsedSections.price ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
              </button>
              {!collapsedSections.price && (
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <Label htmlFor="price-min" className="text-xs text-gray-600">Min</Label>
                    <Input
                      id="price-min"
                      type="number"
                      placeholder="0"
                      value={priceRange.min}
                      onChange={(e) => setPriceRange(prev => ({ ...prev, min: e.target.value }))}
                      className="mt-1"
                    />
                  </div>
                  <div>
                    <Label htmlFor="price-max" className="text-xs text-gray-600">Max</Label>
                    <Input
                      id="price-max"
                      type="number"
                      placeholder="1000"
                      value={priceRange.max}
                      onChange={(e) => setPriceRange(prev => ({ ...prev, max: e.target.value }))}
                      className="mt-1"
                    />
                  </div>
                </div>
              )}
            </div>

            <Separator className="my-4" />

            {/* Expiry Filter */}
            <div className="mb-6">
              <Label className="text-sm font-semibold text-gray-700 mb-3 block">Expiry Status</Label>
              <Select value={expiryDays} onValueChange={setExpiryDays}>
                <SelectTrigger>
                  <SelectValue placeholder="All Products" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Products</SelectItem>
                  <SelectItem value="7">Expires in 7 days</SelectItem>
                  <SelectItem value="30">Expires in 30 days</SelectItem>
                  <SelectItem value="90">Expires in 90 days</SelectItem>
                  <SelectItem value="180">Expires in 6 months</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>

        {/* Main Content Area */}
        <div className="flex-1 overflow-y-auto">
          <div className="p-8">
            {/* Header */}
            <div className="mb-8">
              <h1 className="text-3xl font-bold text-gray-900 mb-2">Pharmaceutical Marketplace</h1>
              <p className="text-gray-600">Browse and procure pharmaceutical products from verified suppliers</p>
            </div>

            {/* Analytics Dashboard */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium text-gray-600">Total Products</CardTitle>
                  <Package className="h-4 w-4 text-gray-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-gray-900">{stats.totalProducts.toLocaleString()}</div>
                  <p className="text-xs text-gray-500 mt-1">Available for procurement</p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium text-gray-600">Total Units</CardTitle>
                  <BarChart3 className="h-4 w-4 text-gray-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-gray-900">{stats.totalQuantity.toLocaleString()}</div>
                  <p className="text-xs text-gray-500 mt-1">In marketplace inventory</p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium text-gray-600">Avg Price</CardTitle>
                  <DollarSign className="h-4 w-4 text-gray-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-gray-900">${stats.averagePrice}</div>
                  <p className="text-xs text-gray-500 mt-1">Per unit average</p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium text-gray-600">Expiring Soon</CardTitle>
                  <AlertTriangle className="h-4 w-4 text-orange-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-orange-600">{stats.expiringSoon}</div>
                  <p className="text-xs text-gray-500 mt-1">Within 30 days</p>
                </CardContent>
              </Card>
            </div>

            {/* Search and Controls */}
            <Card className="mb-6">
              <CardContent className="pt-6">
                <div className="flex flex-col md:flex-row gap-4">
                  <div className="flex-1 relative">
                    <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                    <Input
                      placeholder="Search by product name, manufacturer, or NDC code..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="pl-10"
                    />
                  </div>
                  <div className="flex gap-2">
                    <Select value={sortBy} onValueChange={(value) => setSortBy(value as SortBy)}>
                      <SelectTrigger className="w-48">
                        <SlidersHorizontal className="h-4 w-4 mr-2" />
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="expiry_asc">Expiry: Soonest First</SelectItem>
                        <SelectItem value="expiry_desc">Expiry: Latest First</SelectItem>
                        <SelectItem value="price_asc">Price: Low to High</SelectItem>
                        <SelectItem value="price_desc">Price: High to Low</SelectItem>
                        <SelectItem value="quantity_asc">Quantity: Low to High</SelectItem>
                        <SelectItem value="quantity_desc">Quantity: High to Low</SelectItem>
                        <SelectItem value="name_asc">Name: A to Z</SelectItem>
                        <SelectItem value="name_desc">Name: Z to A</SelectItem>
                      </SelectContent>
                    </Select>
                    <div className="flex border rounded-lg">
                      <Button
                        variant={viewMode === 'list' ? 'default' : 'ghost'}
                        size="sm"
                        onClick={() => setViewMode('list')}
                      >
                        <List className="h-4 w-4" />
                      </Button>
                      <Button
                        variant={viewMode === 'grid' ? 'default' : 'ghost'}
                        size="sm"
                        onClick={() => setViewMode('grid')}
                      >
                        <Grid3x3 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Products List */}
            {filteredInventory.length === 0 ? (
              <Card>
                <CardContent className="py-16">
                  <div className="text-center">
                    <Package className="h-16 w-16 text-gray-300 mx-auto mb-4" />
                    <h3 className="text-xl font-semibold text-gray-900 mb-2">No products found</h3>
                    <p className="text-gray-600 mb-4">Try adjusting your filters or search criteria</p>
                    <Button onClick={clearAllFilters} variant="outline">
                      Clear All Filters
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ) : (
              <>
                {viewMode === 'list' ? (
                  <div className="space-y-4">
                    {paginatedInventory.map((item) => {
                      const daysUntilExpiry = getDaysUntilExpiry(item.expiry_date);
                      const expiryStatus = getExpiryStatus(daysUntilExpiry);
                      const isOwnItem = item.user_id === user?.id;

                      return (
                        <Card key={item.id} className="hover:shadow-lg transition-shadow">
                          <CardContent className="p-6">
                            <div className="flex items-start justify-between">
                              <div className="flex-1">
                                <div className="flex items-start gap-4">
                                  <div className="flex-1">
                                    <h3 className="text-lg font-semibold text-gray-900 mb-1">
                                      {item.pharmaceutical?.brand_name}
                                    </h3>
                                    <p className="text-sm text-gray-600 mb-3">
                                      {item.pharmaceutical?.generic_name} | {item.pharmaceutical?.strength} | {item.pharmaceutical?.dosage_form}
                                    </p>
                                    <div className="flex flex-wrap gap-4 text-sm text-gray-500">
                                      <div className="flex items-center">
                                        <Building className="h-4 w-4 mr-1.5" />
                                        {item.pharmaceutical?.manufacturer}
                                      </div>
                                      <div className="flex items-center">
                                        <Package className="h-4 w-4 mr-1.5" />
                                        Batch: {item.batch_number}
                                      </div>
                                      <div className="flex items-center">
                                        <MapPin className="h-4 w-4 mr-1.5" />
                                        {item.storage_location || 'Location not specified'}
                                      </div>
                                    </div>
                                  </div>
                                  <div className="text-right">
                                    <div className="text-2xl font-bold text-gray-900 mb-1">
                                      ${item.unit_price}
                                    </div>
                                    <p className="text-sm text-gray-500">per unit</p>
                                    <div className="mt-2">
                                      <Badge variant="outline" className="text-xs">
                                        {item.quantity} units available
                                      </Badge>
                                    </div>
                                  </div>
                                </div>
                                <div className="flex items-center justify-between mt-4 pt-4 border-t">
                                  <div className="flex items-center gap-3">
                                    <div className="flex items-center text-sm text-gray-600">
                                      <Calendar className="h-4 w-4 mr-1.5" />
                                      Expires: {new Date(item.expiry_date).toLocaleDateString()}
                                    </div>
                                    <Badge variant={expiryStatus.variant}>
                                      {daysUntilExpiry} days left
                                    </Badge>
                                    <Badge variant="outline" className="text-xs">
                                      Sold by: {item.seller?.company_name || 'Unknown'}
                                    </Badge>
                                  </div>
                                  <div>
                                    {isOwnItem ? (
                                      <Button disabled variant="outline" size="sm">
                                        Your Listing
                                      </Button>
                                    ) : (
                                      <Button onClick={() => openInquiryDialog(item)} size="sm">
                                        <MessageSquare className="h-4 w-4 mr-2" />
                                        Make Inquiry
                                      </Button>
                                    )}
                                  </div>
                                </div>
                              </div>
                            </div>
                          </CardContent>
                        </Card>
                      );
                    })}
                  </div>
                ) : (
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {paginatedInventory.map((item) => {
                      const daysUntilExpiry = getDaysUntilExpiry(item.expiry_date);
                      const expiryStatus = getExpiryStatus(daysUntilExpiry);
                      const isOwnItem = item.user_id === user?.id;

                      return (
                        <Card key={item.id} className="hover:shadow-lg transition-shadow">
                          <CardHeader>
                            <CardTitle className="text-base">{item.pharmaceutical?.brand_name}</CardTitle>
                            <p className="text-sm text-gray-500">{item.pharmaceutical?.generic_name}</p>
                          </CardHeader>
                          <CardContent>
                            <div className="space-y-3">
                              <div className="flex justify-between items-center">
                                <span className="text-2xl font-bold text-gray-900">${item.unit_price}</span>
                                <Badge variant="outline">{item.quantity} units</Badge>
                              </div>
                              <Separator />
                              <div className="space-y-2 text-sm text-gray-600">
                                <div className="flex items-center">
                                  <Building className="h-3.5 w-3.5 mr-2" />
                                  {item.pharmaceutical?.manufacturer}
                                </div>
                                <div className="flex items-center">
                                  <Package className="h-3.5 w-3.5 mr-2" />
                                  {item.pharmaceutical?.dosage_form}
                                </div>
                                <div className="flex items-center justify-between">
                                  <div className="flex items-center">
                                    <Calendar className="h-3.5 w-3.5 mr-2" />
                                    {daysUntilExpiry} days
                                  </div>
                                  <Badge variant={expiryStatus.variant} className="text-xs">
                                    {expiryStatus.label}
                                  </Badge>
                                </div>
                              </div>
                              <Separator />
                              {isOwnItem ? (
                                <Button disabled variant="outline" size="sm" className="w-full">
                                  Your Listing
                                </Button>
                              ) : (
                                <Button onClick={() => openInquiryDialog(item)} size="sm" className="w-full">
                                  <MessageSquare className="h-4 w-4 mr-2" />
                                  Inquire
                                </Button>
                              )}
                            </div>
                          </CardContent>
                        </Card>
                      );
                    })}
                  </div>
                )}

                {/* Pagination */}
                {totalPages > 1 && (
                  <div className="mt-8 flex items-center justify-between">
                    <p className="text-sm text-gray-600">
                      Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredInventory.length)} of {filteredInventory.length} products
                    </p>
                    <div className="flex gap-2">
                      <Button
                        onClick={() => setCurrentPage(prev => Math.max(1, prev - 1))}
                        disabled={currentPage === 1}
                        variant="outline"
                        size="sm"
                      >
                        Previous
                      </Button>
                      <div className="flex items-center gap-2">
                        {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
                          const pageNum = i + 1 + Math.max(0, currentPage - 3);
                          if (pageNum > totalPages) return null;
                          return (
                            <Button
                              key={pageNum}
                              onClick={() => setCurrentPage(pageNum)}
                              variant={currentPage === pageNum ? 'default' : 'outline'}
                              size="sm"
                              className="w-10"
                            >
                              {pageNum}
                            </Button>
                          );
                        })}
                      </div>
                      <Button
                        onClick={() => setCurrentPage(prev => Math.min(totalPages, prev + 1))}
                        disabled={currentPage === totalPages}
                        variant="outline"
                        size="sm"
                      >
                        Next
                      </Button>
                    </div>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>

      {/* Inquiry Dialog */}
      <Dialog open={isInquiryDialogOpen} onOpenChange={setIsInquiryDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Procurement Inquiry</DialogTitle>
          </DialogHeader>
          {selectedItem && (
            <div className="space-y-4">
              <div className="p-4 bg-gray-50 rounded-lg">
                <h4 className="font-semibold text-gray-900 mb-1">{selectedItem.pharmaceutical?.brand_name}</h4>
                <p className="text-sm text-gray-600">{selectedItem.pharmaceutical?.generic_name}</p>
                <div className="mt-2 flex items-center gap-3 text-sm">
                  <span className="font-medium">${selectedItem.unit_price}/unit</span>
                  <span className="text-gray-500">{selectedItem.quantity} available</span>
                </div>
              </div>

              <div>
                <Label htmlFor="inquiry-quantity">Quantity Requested</Label>
                <Input
                  id="inquiry-quantity"
                  type="number"
                  min="1"
                  max={selectedItem.quantity}
                  value={inquiryForm.quantity_requested}
                  onChange={(e) => setInquiryForm(prev => ({ ...prev, quantity_requested: parseInt(e.target.value) || 0 }))}
                  className="mt-1"
                />
              </div>

              <div>
                <Label htmlFor="inquiry-message">Message (Optional)</Label>
                <Textarea
                  id="inquiry-message"
                  rows={4}
                  placeholder="Add any special requirements or questions..."
                  value={inquiryForm.message}
                  onChange={(e) => setInquiryForm(prev => ({ ...prev, message: e.target.value }))}
                  className="mt-1"
                />
              </div>

              <div className="flex gap-3">
                <Button onClick={handleCreateInquiry} className="flex-1">
                  Send Inquiry
                </Button>
                <Button onClick={() => setIsInquiryDialogOpen(false)} variant="outline">
                  Cancel
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </DashboardLayout>
  );
}
