import Papa from 'papaparse';
import * as XLSX from 'xlsx';
import { toast } from 'react-toastify';

/**
 * Export utilities for CSV and Excel file generation
 * Supports pharmaceutical inventory, transactions, and marketplace data
 */

export interface ExportOptions {
  filename?: string;
  sheetName?: string;
}

/**
 * Export data to CSV format
 * @param data - Array of objects to export
 * @param options - Export configuration options
 */
export function exportToCSV<T extends Record<string, unknown>>(
  data: T[],
  options: ExportOptions = {}
): void {
  try {
    if (!data || data.length === 0) {
      toast.error('No data to export');
      return;
    }

    const { filename = `atlas-export-${new Date().toISOString().split('T')[0]}.csv` } = options;

    const csv = Papa.unparse(data);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);

    link.setAttribute('href', url);
    link.setAttribute('download', filename);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    toast.success(`Successfully exported ${data.length} records to CSV`);
  } catch (error) {
    console.error('CSV export error:', error);
    toast.error('Failed to export data to CSV. Please try again.');
  }
}

/**
 * Export data to Excel format
 * @param data - Array of objects to export
 * @param options - Export configuration options
 */
export function exportToExcel<T extends Record<string, unknown>>(
  data: T[],
  options: ExportOptions = {}
): void {
  try {
    if (!data || data.length === 0) {
      toast.error('No data to export');
      return;
    }

    const {
      filename = `atlas-export-${new Date().toISOString().split('T')[0]}.xlsx`,
      sheetName = 'Data'
    } = options;

    const ws = XLSX.utils.json_to_sheet(data);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, sheetName);

    // Auto-size columns based on content
    const maxWidth = 50;
    const colWidths = Object.keys(data[0] || {}).map((key) => {
      const maxLength = Math.max(
        key.length,
        ...data.map((row) => String(row[key] || '').length)
      );
      return { wch: Math.min(maxLength + 2, maxWidth) };
    });
    ws['!cols'] = colWidths;

    XLSX.writeFile(wb, filename);

    toast.success(`Successfully exported ${data.length} records to Excel`);
  } catch (error) {
    console.error('Excel export error:', error);
    toast.error('Failed to export data to Excel. Please try again.');
  }
}

/**
 * Export multiple sheets to a single Excel file
 * @param sheets - Array of sheet configurations with data and names
 * @param filename - Output filename
 */
export function exportToExcelMultiSheet(
  sheets: Array<{ data: Record<string, unknown>[]; sheetName: string }>,
  filename: string = `atlas-export-${new Date().toISOString().split('T')[0]}.xlsx`
): void {
  try {
    if (!sheets || sheets.length === 0) {
      toast.error('No data to export');
      return;
    }

    const wb = XLSX.utils.book_new();

    sheets.forEach(({ data, sheetName }) => {
      if (data && data.length > 0) {
        const ws = XLSX.utils.json_to_sheet(data);

        // Auto-size columns
        const maxWidth = 50;
        const colWidths = Object.keys(data[0] || {}).map((key) => {
          const maxLength = Math.max(
            key.length,
            ...data.map((row) => String(row[key] || '').length)
          );
          return { wch: Math.min(maxLength + 2, maxWidth) };
        });
        ws['!cols'] = colWidths;

        XLSX.utils.book_append_sheet(wb, ws, sheetName);
      }
    });

    if (wb.SheetNames.length === 0) {
      toast.error('No data to export');
      return;
    }

    XLSX.writeFile(wb, filename);

    const totalRecords = sheets.reduce((sum, sheet) => sum + sheet.data.length, 0);
    toast.success(`Successfully exported ${totalRecords} records to Excel`);
  } catch (error) {
    console.error('Multi-sheet Excel export error:', error);
    toast.error('Failed to export data to Excel. Please try again.');
  }
}

/**
 * Format pharmaceutical inventory data for export
 */
export function formatInventoryForExport(inventory: any[]): Record<string, unknown>[] {
  return inventory.map((item) => ({
    'Product Name': item.pharmaceutical?.brand_name || 'N/A',
    'Generic Name': item.pharmaceutical?.generic_name || 'N/A',
    'NDC Code': item.pharmaceutical?.ndc_code || 'N/A',
    'Manufacturer': item.pharmaceutical?.manufacturer || 'N/A',
    'Batch Number': item.batch_number || 'N/A',
    'Quantity': item.quantity || 0,
    'Unit Price': `$${parseFloat(item.unit_price || 0).toFixed(2)}`,
    'Total Value': `$${(parseFloat(item.unit_price || 0) * (item.quantity || 0)).toFixed(2)}`,
    'Expiry Date': item.expiry_date ? new Date(item.expiry_date).toLocaleDateString() : 'N/A',
    'Status': item.status || 'N/A',
    'Location': item.location || 'N/A',
    'Storage Conditions': item.storage_conditions || 'N/A',
    'Created At': item.created_at ? new Date(item.created_at).toLocaleDateString() : 'N/A',
  }));
}

/**
 * Format pharmaceutical catalog data for export
 */
export function formatPharmaceuticalsForExport(pharmaceuticals: any[]): Record<string, unknown>[] {
  return pharmaceuticals.map((item) => ({
    'Brand Name': item.brand_name || 'N/A',
    'Generic Name': item.generic_name || 'N/A',
    'NDC Code': item.ndc_code || 'N/A',
    'Manufacturer': item.manufacturer || 'N/A',
    'Dosage Form': item.dosage_form || 'N/A',
    'Strength': item.strength || 'N/A',
    'Category': item.category || 'N/A',
    'Created At': item.created_at ? new Date(item.created_at).toLocaleDateString() : 'N/A',
  }));
}

/**
 * Format transaction data for export
 */
export function formatTransactionsForExport(transactions: any[]): Record<string, unknown>[] {
  return transactions.map((txn) => ({
    'Transaction ID': txn.id || 'N/A',
    'Product': txn.inventory?.pharmaceutical?.brand_name || 'N/A',
    'Batch Number': txn.inventory?.batch_number || 'N/A',
    'Buyer': txn.buyer?.company_name || 'N/A',
    'Seller': txn.seller?.company_name || 'N/A',
    'Quantity': txn.quantity || 0,
    'Unit Price': `$${parseFloat(txn.unit_price || 0).toFixed(2)}`,
    'Total Amount': `$${parseFloat(txn.total_amount || 0).toFixed(2)}`,
    'Status': txn.status || 'N/A',
    'Created At': txn.created_at ? new Date(txn.created_at).toLocaleDateString() : 'N/A',
  }));
}

/**
 * Format inquiry data for export
 */
export function formatInquiriesForExport(inquiries: any[]): Record<string, unknown>[] {
  return inquiries.map((inq) => ({
    'Inquiry ID': inq.id || 'N/A',
    'Product': inq.inventory?.pharmaceutical?.brand_name || 'N/A',
    'Batch Number': inq.inventory?.batch_number || 'N/A',
    'Buyer': inq.buyer?.company_name || 'N/A',
    'Quantity Requested': inq.quantity || 0,
    'Message': inq.message || 'N/A',
    'Status': inq.status || 'N/A',
    'Created At': inq.created_at ? new Date(inq.created_at).toLocaleDateString() : 'N/A',
  }));
}

/**
 * Format analytics data for export
 */
export function formatAnalyticsForExport(
  stats: Record<string, unknown>
): Record<string, unknown>[] {
  return [
    {
      'Metric': 'Total Inventory Items',
      'Value': stats.totalInventory || 0,
    },
    {
      'Metric': 'Active Listings',
      'Value': stats.activeListings || 0,
    },
    {
      'Metric': 'Total Value',
      'Value': `$${Number(stats.totalValue || 0).toLocaleString()}`,
    },
    {
      'Metric': 'Average Price',
      'Value': `$${Number(stats.averagePrice || 0).toFixed(2)}`,
    },
    {
      'Metric': 'Low Stock Items',
      'Value': stats.lowStockItems || 0,
    },
    {
      'Metric': 'Expiring Items (30 days)',
      'Value': stats.expiringItems || 0,
    },
    {
      'Metric': 'Pending Inquiries',
      'Value': stats.pendingInquiries || 0,
    },
    {
      'Metric': 'Total Transactions',
      'Value': stats.totalTransactions || 0,
    },
    {
      'Metric': 'Stock Utilization',
      'Value': `${Number(stats.stockUtilization || 0).toFixed(1)}%`,
    },
  ];
}
