import { z } from 'zod';

/**
 * Validation schemas for Atlas Pharma forms
 * Using Zod for runtime type checking and validation
 */

// ============================================================================
// Authentication Schemas
// ============================================================================

export const loginSchema = z.object({
  email: z
    .string()
    .min(1, 'Email is required')
    .email('Invalid email address'),
  password: z
    .string()
    .min(1, 'Password is required')
    .min(8, 'Password must be at least 8 characters'),
});

export const registerSchema = z.object({
  email: z
    .string()
    .min(1, 'Email is required')
    .email('Invalid email address'),
  password: z
    .string()
    .min(8, 'Password must be at least 8 characters')
    .regex(/[A-Z]/, 'Password must contain at least one uppercase letter')
    .regex(/[a-z]/, 'Password must contain at least one lowercase letter')
    .regex(/[0-9]/, 'Password must contain at least one number'),
  confirmPassword: z
    .string()
    .min(1, 'Please confirm your password'),
  company_name: z
    .string()
    .min(1, 'Company name is required')
    .min(2, 'Company name must be at least 2 characters'),
  company_type: z.enum(['manufacturer', 'distributor', 'pharmacy', 'hospital'], {
    errorMap: () => ({ message: 'Please select a valid company type' }),
  }),
  license_number: z
    .string()
    .min(1, 'License number is required'),
  contact_person: z
    .string()
    .min(1, 'Contact person is required'),
  phone: z
    .string()
    .min(1, 'Phone number is required')
    .regex(/^[\d\s\-\+\(\)]+$/, 'Invalid phone number format'),
  address: z
    .string()
    .min(1, 'Address is required'),
  city: z
    .string()
    .min(1, 'City is required'),
  state: z
    .string()
    .min(1, 'State is required'),
  zip_code: z
    .string()
    .min(1, 'ZIP code is required')
    .regex(/^\d{5}(-\d{4})?$/, 'Invalid ZIP code format'),
}).refine((data) => data.password === data.confirmPassword, {
  message: "Passwords don't match",
  path: ['confirmPassword'],
});

// ============================================================================
// Pharmaceutical Schemas
// ============================================================================

export const pharmaceuticalSchema = z.object({
  brand_name: z
    .string()
    .min(1, 'Brand name is required')
    .max(100, 'Brand name must be less than 100 characters'),
  generic_name: z
    .string()
    .min(1, 'Generic name is required')
    .max(100, 'Generic name must be less than 100 characters'),
  ndc_code: z
    .string()
    .optional()
    .refine((val) => !val || /^\d{5}-\d{4}-\d{2}$/.test(val), {
      message: 'NDC code must be in format: 12345-6789-01',
    }),
  manufacturer: z
    .string()
    .min(1, 'Manufacturer is required'),
  category: z
    .string()
    .min(1, 'Category is required'),
  description: z
    .string()
    .optional(),
  strength: z
    .string()
    .optional(),
  dosage_form: z
    .string()
    .optional(),
  storage_requirements: z
    .string()
    .optional(),
});

// ============================================================================
// Inventory Schemas
// ============================================================================

export const inventorySchema = z.object({
  pharmaceutical_id: z
    .string()
    .min(1, 'Please select a pharmaceutical product'),
  batch_number: z
    .string()
    .min(1, 'Batch number is required')
    .max(50, 'Batch number must be less than 50 characters'),
  quantity: z
    .number()
    .int('Quantity must be a whole number')
    .positive('Quantity must be greater than 0')
    .max(1000000, 'Quantity seems unreasonably high'),
  expiry_date: z
    .string()
    .min(1, 'Expiry date is required')
    .refine((date) => {
      const expiryDate = new Date(date);
      const today = new Date();
      return expiryDate > today;
    }, {
      message: 'Expiry date must be in the future',
    }),
  unit_price: z
    .string()
    .min(1, 'Unit price is required')
    .refine((val) => {
      const num = parseFloat(val);
      return !isNaN(num) && num > 0;
    }, {
      message: 'Unit price must be a positive number',
    }),
  storage_location: z
    .string()
    .optional(),
});

export const updateInventorySchema = z.object({
  quantity: z
    .number()
    .int('Quantity must be a whole number')
    .nonnegative('Quantity cannot be negative')
    .max(1000000, 'Quantity seems unreasonably high'),
  unit_price: z
    .string()
    .refine((val) => {
      const num = parseFloat(val);
      return !isNaN(num) && num > 0;
    }, {
      message: 'Unit price must be a positive number',
    }),
  storage_location: z
    .string()
    .optional(),
});

// ============================================================================
// Marketplace Schemas
// ============================================================================

export const inquirySchema = z.object({
  inventory_id: z
    .string()
    .min(1, 'Please select an inventory item'),
  quantity: z
    .number()
    .int('Quantity must be a whole number')
    .positive('Quantity must be greater than 0'),
  message: z
    .string()
    .min(10, 'Message must be at least 10 characters')
    .max(500, 'Message must be less than 500 characters'),
});

export const transactionSchema = z.object({
  inquiry_id: z
    .string()
    .min(1, 'Inquiry ID is required'),
  quantity: z
    .number()
    .int('Quantity must be a whole number')
    .positive('Quantity must be greater than 0'),
  unit_price: z
    .string()
    .refine((val) => {
      const num = parseFloat(val);
      return !isNaN(num) && num > 0;
    }, {
      message: 'Unit price must be a positive number',
    }),
});

// ============================================================================
// Type Exports (inferred from schemas)
// ============================================================================

export type LoginFormData = z.infer<typeof loginSchema>;
export type RegisterFormData = z.infer<typeof registerSchema>;
export type PharmaceuticalFormData = z.infer<typeof pharmaceuticalSchema>;
export type InventoryFormData = z.infer<typeof inventorySchema>;
export type UpdateInventoryFormData = z.infer<typeof updateInventorySchema>;
export type InquiryFormData = z.infer<typeof inquirySchema>;
export type TransactionFormData = z.infer<typeof transactionSchema>;

// ============================================================================
// Validation Helper Functions
// ============================================================================

/**
 * Validates data against a schema and returns either the validated data or errors
 */
export function validate<T>(schema: z.ZodSchema<T>, data: unknown): {
  success: boolean;
  data?: T;
  errors?: Record<string, string>;
} {
  try {
    const validatedData = schema.parse(data);
    return {
      success: true,
      data: validatedData,
    };
  } catch (error) {
    if (error instanceof z.ZodError) {
      const errors: Record<string, string> = {};
      error.errors.forEach((err) => {
        if (err.path.length > 0) {
          errors[err.path.join('.')] = err.message;
        }
      });
      return {
        success: false,
        errors,
      };
    }
    throw error;
  }
}

/**
 * Validates a single field against a schema
 */
export function validateField<T>(
  schema: z.ZodSchema<T>,
  fieldName: string,
  value: unknown,
  allData?: Partial<T>
): string | null {
  try {
    const dataToValidate = allData ? { ...allData, [fieldName]: value } : { [fieldName]: value };
    schema.parse(dataToValidate);
    return null;
  } catch (error) {
    if (error instanceof z.ZodError) {
      const fieldError = error.errors.find((err) => err.path.includes(fieldName));
      return fieldError?.message || null;
    }
    return null;
  }
}
