'use client';

import { useState, useEffect } from 'react';
import { DashboardLayout } from '@/components/dashboard-layout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
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
  Plus,
  Search,
  Filter,
  Pill,
  Building,
  Package,
  Edit,
  Trash2,
  Eye,
  Download,
  FileText,
  Grid3x3
} from 'lucide-react';
import { PharmaceuticalService, OpenFdaService } from '@/lib/services';
import {
  Pharmaceutical,
  CreatePharmaceuticalRequest,
  Manufacturer,
  Category
} from '@/types/pharmaceutical';
import { toast } from 'react-toastify';
import { exportToCSV, exportToExcel, formatPharmaceuticalsForExport } from '@/lib/utils/export';
import type { OpenFdaDrug } from '@/lib/services/openfda-service';

export default function PharmaceuticalsPage() {
  const [pharmaceuticals, setPharmaceuticals] = useState<Pharmaceutical[]>([]);
  const [manufacturers, setManufacturers] = useState<Manufacturer[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isAddDialogOpen, setIsAddDialogOpen] = useState(false);
  const [isFdaImportOpen, setIsFdaImportOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedManufacturer, setSelectedManufacturer] = useState<string>('');
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const [fdaSearchTerm, setFdaSearchTerm] = useState('');
  const [fdaDrugs, setFdaDrugs] = useState<OpenFdaDrug[]>([]);
  const [isFdaLoading, setIsFdaLoading] = useState(false);
  const [formData, setFormData] = useState<CreatePharmaceuticalRequest>({
    brand_name: '',
    generic_name: '',
    ndc_code: '',
    manufacturer: '',
    category: '',
    description: '',
    strength: '',
    dosage_form: '',
    storage_requirements: '',
  });

  useEffect(() => {
    loadData();
  }, [searchTerm, selectedManufacturer, selectedCategory]);

  const loadData = async () => {
    try {
      setIsLoading(true);

      // Load pharmaceuticals with filters
      const searchParams: any = {
        search: searchTerm || undefined,
        manufacturer: (selectedManufacturer && selectedManufacturer !== 'all') ? selectedManufacturer : undefined,
        category: (selectedCategory && selectedCategory !== 'all') ? selectedCategory : undefined,
        limit: 100,
      };

      const [pharmaData, manufacturersData, categoriesData] = await Promise.all([
        PharmaceuticalService.searchPharmaceuticals(searchParams),
        PharmaceuticalService.getManufacturers(),
        PharmaceuticalService.getCategories(),
      ]);

      setPharmaceuticals(pharmaData);
      setManufacturers(manufacturersData);
      setCategories(categoriesData);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to load pharmaceuticals');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreatePharmaceutical = async () => {
    try {
      const newPharma = await PharmaceuticalService.createPharmaceutical(formData);
      setPharmaceuticals([newPharma, ...pharmaceuticals]);
      setIsAddDialogOpen(false);
      setFormData({
        brand_name: '',
        generic_name: '',
        ndc_code: '',
        manufacturer: '',
        category: '',
        description: '',
        strength: '',
        dosage_form: '',
        storage_requirements: '',
      });
      toast.success('Pharmaceutical added successfully');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to add pharmaceutical');
    }
  };

  const handleFdaSearch = async () => {
    if (!fdaSearchTerm.trim()) {
      toast.error('Please enter a search term');
      return;
    }

    try {
      setIsFdaLoading(true);
      const results = await OpenFdaService.search({
        query: fdaSearchTerm,
        limit: 20
      });
      setFdaDrugs(results);
      if (results.length === 0) {
        toast.info('No results found. The FDA catalog may need to be synced.');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to search FDA catalog');
    } finally {
      setIsFdaLoading(false);
    }
  };

  const handleImportFromFda = async (drug: OpenFdaDrug) => {
    try {
      const pharmaData: CreatePharmaceuticalRequest = {
        brand_name: drug.brand_name,
        generic_name: drug.generic_name,
        ndc_code: drug.product_ndc,
        manufacturer: drug.labeler_name,
        category: drug.marketing_category || 'Prescription',
        description: `${drug.brand_name} - ${drug.generic_name}`,
        strength: drug.strength || '',
        dosage_form: drug.dosage_form || '',
        storage_requirements: '',
      };

      const newPharma = await PharmaceuticalService.createPharmaceutical(pharmaData);
      setPharmaceuticals([newPharma, ...pharmaceuticals]);
      toast.success(`Imported ${drug.brand_name} successfully`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to import from FDA catalog');
    }
  };

  const filteredPharmaceuticals = pharmaceuticals.filter(pharma => {
    const matchesSearch =
      !searchTerm ||
      pharma.brand_name?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      pharma.generic_name?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      pharma.ndc_code?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      pharma.manufacturer?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      pharma.category?.toLowerCase().includes(searchTerm.toLowerCase());

    const matchesManufacturer = !selectedManufacturer || selectedManufacturer === 'all' || pharma.manufacturer === selectedManufacturer;
    const matchesCategory = !selectedCategory || selectedCategory === 'all' || pharma.category === selectedCategory;

    return matchesSearch && matchesManufacturer && matchesCategory;
  });

  const handleExportCSV = () => {
    const exportData = formatPharmaceuticalsForExport(filteredPharmaceuticals);
    exportToCSV(exportData, {
      filename: `pharmaceuticals-export-${new Date().toISOString().split('T')[0]}.csv`
    });
  };

  const handleExportExcel = () => {
    const exportData = formatPharmaceuticalsForExport(filteredPharmaceuticals);
    exportToExcel(exportData, {
      filename: `pharmaceuticals-export-${new Date().toISOString().split('T')[0]}.xlsx`,
      sheetName: 'Pharmaceuticals'
    });
  };

  return (
    <ProtectedRoute requireVerification={true}>
      <DashboardLayout>
        <div className="p-6 space-y-6">
          {/* Header */}
          <div className="flex items-center justify-between flex-wrap gap-3">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">Pharmaceutical Catalog</h1>
              <p className="text-gray-600">Manage the pharmaceutical product catalog</p>
            </div>
            <div className="flex items-center gap-3">
              <Button variant="outline" onClick={handleExportCSV} disabled={filteredPharmaceuticals.length === 0}>
                <FileText className="h-4 w-4 mr-2" />
                Export CSV
              </Button>
              <Button variant="outline" onClick={handleExportExcel} disabled={filteredPharmaceuticals.length === 0}>
                <Grid3x3 className="h-4 w-4 mr-2" />
                Export Excel
              </Button>
              <Dialog open={isFdaImportOpen} onOpenChange={setIsFdaImportOpen}>
                <DialogTrigger asChild>
                  <Button variant="outline">
                    <Download className="h-4 w-4 mr-2" />
                    Import from FDA
                  </Button>
                </DialogTrigger>
                <DialogContent className="sm:max-w-4xl max-h-[90vh] overflow-y-auto">
                  <DialogHeader>
                    <DialogTitle>Import from FDA Catalog</DialogTitle>
                  </DialogHeader>
                  <div className="space-y-4">
                    <div className="flex gap-2">
                      <Input
                        placeholder="Search FDA catalog (e.g., aspirin, amoxicillin)..."
                        value={fdaSearchTerm}
                        onChange={(e) => setFdaSearchTerm(e.target.value)}
                        onKeyPress={(e) => e.key === 'Enter' && handleFdaSearch()}
                        className="flex-1"
                      />
                      <Button onClick={handleFdaSearch} disabled={isFdaLoading}>
                        <Search className="h-4 w-4 mr-2" />
                        {isFdaLoading ? 'Searching...' : 'Search'}
                      </Button>
                    </div>

                    {fdaDrugs.length > 0 ? (
                      <div className="border rounded-lg overflow-hidden">
                        <Table>
                          <TableHeader>
                            <TableRow>
                              <TableHead>Brand Name</TableHead>
                              <TableHead>Generic Name</TableHead>
                              <TableHead>Manufacturer</TableHead>
                              <TableHead>NDC</TableHead>
                              <TableHead>Form</TableHead>
                              <TableHead className="text-right">Action</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {fdaDrugs.map((drug) => (
                              <TableRow key={drug.id}>
                                <TableCell className="font-medium">{drug.brand_name}</TableCell>
                                <TableCell>{drug.generic_name}</TableCell>
                                <TableCell>{drug.labeler_name}</TableCell>
                                <TableCell className="font-mono text-sm">{drug.product_ndc}</TableCell>
                                <TableCell>{drug.dosage_form || '-'}</TableCell>
                                <TableCell className="text-right">
                                  <Button
                                    size="sm"
                                    onClick={() => handleImportFromFda(drug)}
                                  >
                                    <Download className="h-3 w-3 mr-1" />
                                    Import
                                  </Button>
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </div>
                    ) : (
                      <div className="text-center py-8 text-gray-500">
                        <p>Search the FDA catalog to find drugs to import</p>
                        <p className="text-sm mt-2">The catalog contains FDA-approved pharmaceuticals</p>
                      </div>
                    )}
                  </div>
                </DialogContent>
              </Dialog>
              <Dialog open={isAddDialogOpen} onOpenChange={setIsAddDialogOpen}>
                <DialogTrigger asChild>
                  <Button>
                    <Plus className="h-4 w-4 mr-2" />
                    Add Pharmaceutical
                  </Button>
                </DialogTrigger>
              <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                  <DialogTitle>Add New Pharmaceutical</DialogTitle>
                </DialogHeader>
                <div className="space-y-4">
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <Label htmlFor="brand_name">Brand Name *</Label>
                      <Input
                        id="brand_name"
                        value={formData.brand_name}
                        onChange={(e) => setFormData({ ...formData, brand_name: e.target.value })}
                        placeholder="Enter brand name"
                      />
                    </div>
                    <div>
                      <Label htmlFor="generic_name">Generic Name *</Label>
                      <Input
                        id="generic_name"
                        value={formData.generic_name}
                        onChange={(e) => setFormData({ ...formData, generic_name: e.target.value })}
                        placeholder="Enter generic name"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <Label htmlFor="ndc_code">NDC Code</Label>
                      <Input
                        id="ndc_code"
                        value={formData.ndc_code}
                        onChange={(e) => setFormData({ ...formData, ndc_code: e.target.value })}
                        placeholder="Enter NDC code"
                      />
                    </div>
                    <div>
                      <Label htmlFor="manufacturer">Manufacturer *</Label>
                      <Input
                        id="manufacturer"
                        value={formData.manufacturer}
                        onChange={(e) => setFormData({ ...formData, manufacturer: e.target.value })}
                        placeholder="Enter manufacturer name"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-4">
                    <div>
                      <Label htmlFor="category">Category</Label>
                      <Select
                        value={formData.category}
                        onValueChange={(value) => setFormData({ ...formData, category: value })}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="Select category" />
                        </SelectTrigger>
                        <SelectContent>
                          {categories.filter(cat => cat.category && cat.category.trim() !== '').map((category) => (
                            <SelectItem key={category.category} value={category.category}>
                              {category.category}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div>
                      <Label htmlFor="strength">Strength</Label>
                      <Input
                        id="strength"
                        value={formData.strength}
                        onChange={(e) => setFormData({ ...formData, strength: e.target.value })}
                        placeholder="e.g., 500mg"
                      />
                    </div>
                    <div>
                      <Label htmlFor="dosage_form">Dosage Form</Label>
                      <Select
                        value={formData.dosage_form}
                        onValueChange={(value) => setFormData({ ...formData, dosage_form: value })}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="Select form" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="Tablet">Tablet</SelectItem>
                          <SelectItem value="Capsule">Capsule</SelectItem>
                          <SelectItem value="Liquid">Liquid</SelectItem>
                          <SelectItem value="Injection">Injection</SelectItem>
                          <SelectItem value="Cream">Cream</SelectItem>
                          <SelectItem value="Ointment">Ointment</SelectItem>
                          <SelectItem value="Inhaler">Inhaler</SelectItem>
                          <SelectItem value="Patch">Patch</SelectItem>
                          <SelectItem value="Other">Other</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>

                  <div>
                    <Label htmlFor="description">Description</Label>
                    <Textarea
                      id="description"
                      value={formData.description}
                      onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                      placeholder="Enter product description"
                      rows={3}
                    />
                  </div>

                  <div>
                    <Label htmlFor="storage_requirements">Storage Requirements</Label>
                    <Textarea
                      id="storage_requirements"
                      value={formData.storage_requirements}
                      onChange={(e) => setFormData({ ...formData, storage_requirements: e.target.value })}
                      placeholder="Enter storage requirements (e.g., Store at room temperature, protect from light)"
                      rows={2}
                    />
                  </div>

                  <Button onClick={handleCreatePharmaceutical} className="w-full">
                    Add Pharmaceutical
                  </Button>
                </div>
              </DialogContent>
            </Dialog>
            </div>
          </div>

          {/* Search and Filters */}
          <Card>
            <CardHeader>
              <CardTitle>Search & Filters</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="md:col-span-1">
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                    <Input
                      placeholder="Search by name, generic, or NDC..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="pl-10"
                    />
                  </div>
                </div>
                <Select value={selectedManufacturer} onValueChange={setSelectedManufacturer}>
                  <SelectTrigger>
                    <SelectValue placeholder="All Manufacturers" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Manufacturers</SelectItem>
                    {manufacturers.filter(mfg => mfg.manufacturer && mfg.manufacturer.trim() !== '').map((mfg) => (
                      <SelectItem key={mfg.manufacturer} value={mfg.manufacturer}>
                        {mfg.manufacturer} ({mfg.count})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Select value={selectedCategory} onValueChange={setSelectedCategory}>
                  <SelectTrigger>
                    <SelectValue placeholder="All Categories" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Categories</SelectItem>
                    {categories.filter(cat => cat.category && cat.category.trim() !== '').map((category) => (
                      <SelectItem key={category.category} value={category.category}>
                        {category.category} ({category.count})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          {/* Statistics Cards */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Total Products</CardTitle>
                <Pill className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{filteredPharmaceuticals.length}</div>
                <p className="text-xs text-muted-foreground">
                  In catalog
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Manufacturers</CardTitle>
                <Building className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{manufacturers.length}</div>
                <p className="text-xs text-muted-foreground">
                  Total suppliers
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">Categories</CardTitle>
                <Package className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{categories.length}</div>
                <p className="text-xs text-muted-foreground">
                  Product types
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Products Table */}
          <Card>
            <CardHeader>
              <CardTitle>Products ({filteredPharmaceuticals.length})</CardTitle>
            </CardHeader>
            <CardContent>
              {filteredPharmaceuticals.length === 0 ? (
                <div className="text-center py-8">
                  <Pill className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                  <h3 className="text-lg font-medium text-gray-900 mb-2">No pharmaceuticals found</h3>
                  <p className="text-gray-600 mb-4">
                    {searchTerm || selectedManufacturer || selectedCategory
                      ? 'Try adjusting your search filters'
                      : 'Get started by adding your first pharmaceutical product'
                    }
                  </p>
                  {!searchTerm && !selectedManufacturer && !selectedCategory && (
                    <Button onClick={() => setIsAddDialogOpen(true)}>
                      <Plus className="h-4 w-4 mr-2" />
                      Add First Product
                    </Button>
                  )}
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Product Information</TableHead>
                        <TableHead>Manufacturer</TableHead>
                        <TableHead>Category</TableHead>
                        <TableHead>Strength & Form</TableHead>
                        <TableHead>NDC Code</TableHead>
                        <TableHead>Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {filteredPharmaceuticals.map((pharma) => (
                        <TableRow key={pharma.id}>
                          <TableCell>
                            <div>
                              <div className="font-medium text-gray-900">
                                {pharma.brand_name}
                              </div>
                              <div className="text-sm text-gray-500">
                                {pharma.generic_name}
                              </div>
                              {pharma.description && (
                                <div className="text-xs text-gray-400 mt-1 max-w-xs truncate">
                                  {pharma.description}
                                </div>
                              )}
                            </div>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center">
                              <Building className="h-4 w-4 mr-2 text-gray-400" />
                              {pharma.manufacturer}
                            </div>
                          </TableCell>
                          <TableCell>
                            {pharma.category ? (
                              <Badge variant="outline">{pharma.category}</Badge>
                            ) : (
                              <span className="text-gray-400">-</span>
                            )}
                          </TableCell>
                          <TableCell>
                            <div className="text-sm">
                              <div>{pharma.strength || '-'}</div>
                              <div className="text-gray-500">{pharma.dosage_form || '-'}</div>
                            </div>
                          </TableCell>
                          <TableCell className="font-mono text-sm">
                            {pharma.ndc_code || '-'}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center space-x-2">
                              <Button variant="ghost" size="sm">
                                <Eye className="h-4 w-4" />
                              </Button>
                            </div>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </DashboardLayout>
    </ProtectedRoute>
  );
}