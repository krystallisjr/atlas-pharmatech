/**
 * Environment Variable Validation and Type Safety
 *
 * This module ensures that all required environment variables are present
 * and provides type-safe access to them throughout the application.
 */

interface EnvironmentVariables {
  NEXT_PUBLIC_API_URL: string;
  NEXT_PUBLIC_APP_NAME: string;
  NODE_ENV: 'development' | 'production' | 'test';
}

/**
 * Validates that a required environment variable exists
 */
function requireEnv(key: string, defaultValue?: string): string {
  const value = process.env[key] || defaultValue;

  if (!value) {
    throw new Error(
      `Missing required environment variable: ${key}\n` +
      `Please add it to your .env.local file.`
    );
  }

  return value;
}

/**
 * Validates all required environment variables on app startup
 */
export function validateEnvironment(): void {
  const required = ['NEXT_PUBLIC_API_URL'];
  const missing: string[] = [];

  for (const key of required) {
    if (!process.env[key]) {
      missing.push(key);
    }
  }

  if (missing.length > 0) {
    throw new Error(
      `Missing required environment variables:\n` +
      missing.map((key) => `  - ${key}`).join('\n') +
      `\n\nPlease add them to your .env.local file.`
    );
  }
}

/**
 * Type-safe environment variables with defaults
 */
export const env: EnvironmentVariables = {
  NEXT_PUBLIC_API_URL: requireEnv('NEXT_PUBLIC_API_URL', 'http://localhost:8080'),
  NEXT_PUBLIC_APP_NAME: requireEnv('NEXT_PUBLIC_APP_NAME', 'Atlas PharmaTech'),
  NODE_ENV: (process.env.NODE_ENV as 'development' | 'production' | 'test') || 'development',
};

/**
 * Check if we're in development mode
 */
export const isDevelopment = env.NODE_ENV === 'development';

/**
 * Check if we're in production mode
 */
export const isProduction = env.NODE_ENV === 'production';

/**
 * Check if we're in test mode
 */
export const isTest = env.NODE_ENV === 'test';

/**
 * Get the API base URL
 */
export function getApiBaseUrl(): string {
  return env.NEXT_PUBLIC_API_URL;
}

/**
 * Get the app name
 */
export function getAppName(): string {
  return env.NEXT_PUBLIC_APP_NAME;
}

/**
 * Logs environment configuration (safe for development only)
 */
export function logEnvironment(): void {
  if (isDevelopment) {
    console.log('🔧 Environment Configuration:');
    console.log(`  - NODE_ENV: ${env.NODE_ENV}`);
    console.log(`  - API URL: ${env.NEXT_PUBLIC_API_URL}`);
    console.log(`  - App Name: ${env.NEXT_PUBLIC_APP_NAME}`);
  }
}

// Validate environment on module load (only in browser)
if (typeof window !== 'undefined') {
  try {
    validateEnvironment();
    if (isDevelopment) {
      logEnvironment();
    }
  } catch (error) {
    console.error('❌ Environment Validation Error:', error);
    // In development, we can show a more helpful error
    if (isDevelopment) {
      throw error;
    }
  }
}
