# Atlas PharmaTech Frontend 🏥

A production-ready pharmaceutical inventory management and marketplace platform built with Next.js 15, TypeScript, and modern web technologies.

## ✨ Features

### Core Functionality
- **🔐 User Authentication**: Secure JWT-based authentication with role-based access
- **📊 Advanced Dashboard**: Interactive analytics with real-time charts and insights
- **💊 Pharmaceutical Catalog**: Complete product management with NDC code support
- **📦 Inventory Management**: Full CRUD operations with expiry tracking and batch management
- **🛒 Marketplace Trading**: Browse, search, and trade pharmaceutical products
- **💬 Inquiry System**: Buyer-seller communication with status tracking
- **💳 Transaction Management**: Complete transaction lifecycle and history
- **📈 Business Analytics**: Comprehensive insights with category distribution, trends, and forecasting

### Production-Ready Features
- **📥 Data Export**: CSV and Excel export for inventory, pharmaceuticals, and transactions
- **📱 QR Code Generation**: Generate QR codes for products, batches, and quick access
- **🔍 Advanced Search**: Multi-parameter search with filters and autocomplete
- **⚡ API Status Monitoring**: Real-time backend health monitoring and endpoint status
- **🎨 Loading States**: Skeleton screens for better UX
- **🛡️ Error Boundaries**: Graceful error handling with detailed feedback
- **✅ Form Validation**: Zod-based validation with helpful error messages
- **🌓 Responsive Design**: Mobile-first design that works on all devices
- **🚨 Smart Alerts**: Expiry warnings, low stock alerts, and batch tracking
- **🔔 Toast Notifications**: Real-time feedback for all user actions

## 🚀 Tech Stack

### Frontend Framework
- **Next.js 15**: React framework with App Router and Server Components
- **React 18**: Latest React with hooks and concurrent features
- **TypeScript**: Full type safety throughout the application

### UI & Styling
- **Tailwind CSS**: Utility-first CSS framework
- **shadcn/ui**: Beautiful, accessible component library
- **Lucide React**: Modern icon library
- **Recharts**: Interactive charts and data visualization

### Data & State Management
- **Axios**: HTTP client with interceptors and error handling
- **React Context**: Centralized auth state management
- **React Hook Form**: Performant form handling
- **Zod**: TypeScript-first schema validation

### Utilities & Tools
- **QRCode**: QR code generation for products and inventory
- **PapaParse**: CSV parsing and generation
- **XLSX**: Excel file generation and export
- **React Toastify**: Beautiful toast notifications

## 📋 Prerequisites

- **Node.js**: v16.0.0 or higher
- **npm** or **yarn**: Latest version
- **Atlas PharmaTech Backend**: Running on `http://localhost:8080`

## 🛠️ Installation & Setup

### 1. Clone & Navigate
```bash
cd atlas-frontend
```

### 2. Install Dependencies
```bash
npm install
```

### 3. Environment Configuration
Create a `.env.local` file in the root directory:

```env
# Required
NEXT_PUBLIC_API_URL=http://localhost:8080

# Optional
NEXT_PUBLIC_APP_NAME="Atlas PharmaTech"
```

### 4. Start Development Server
```bash
npm run dev
```

The application will be available at `http://localhost:3000`

## 🏗️ Project Structure

```
atlas-frontend/
├── src/
│   ├── app/                    # Next.js App Router pages
│   │   ├── dashboard/          # Dashboard pages
│   │   │   ├── page.tsx        # Main dashboard with analytics
│   │   │   ├── inventory/      # Inventory management
│   │   │   ├── pharmaceuticals/ # Product catalog
│   │   │   ├── marketplace/    # Marketplace listings
│   │   │   ├── inquiries/      # Buyer/seller inquiries
│   │   │   ├── transactions/   # Transaction history
│   │   │   └── api-status/     # API health monitoring
│   │   ├── login/              # Authentication pages
│   │   ├── register/
│   │   └── layout.tsx          # Root layout
│   │
│   ├── components/             # Reusable components
│   │   ├── ui/                 # shadcn/ui components
│   │   │   ├── analytics-card.tsx
│   │   │   ├── chart-card.tsx
│   │   │   ├── qr-code.tsx
│   │   │   ├── skeleton.tsx
│   │   │   └── loading-skeletons.tsx
│   │   ├── dashboard-layout.tsx
│   │   ├── error-boundary.tsx
│   │   └── protected-route.tsx
│   │
│   ├── lib/                    # Core utilities
│   │   ├── services/           # API service layer
│   │   │   ├── auth-service.ts
│   │   │   ├── inventory-service.ts
│   │   │   ├── marketplace-service.ts
│   │   │   └── pharmaceutical-service.ts
│   │   ├── utils/              # Utility functions
│   │   │   ├── export.ts       # CSV/Excel export
│   │   │   └── env.ts          # Environment validation
│   │   ├── validation/         # Zod schemas
│   │   │   └── schemas.ts
│   │   ├── api-client.ts       # Axios configuration
│   │   └── utils.ts            # Helper functions
│   │
│   ├── contexts/               # React contexts
│   │   └── auth-context.tsx    # Authentication context
│   │
│   └── types/                  # TypeScript definitions
│       ├── auth.ts
│       ├── pharmaceutical.ts
│       └── api.ts
│
├── public/                     # Static assets
├── .env.local                  # Environment variables
├── next.config.js              # Next.js configuration
├── tailwind.config.ts          # Tailwind configuration
├── tsconfig.json               # TypeScript configuration
└── package.json                # Dependencies
```

## 🎯 Key Features Explained

### Data Export
Export your data to CSV or Excel format with one click:
- **Inventory**: Export with pharmaceutical details, batch numbers, expiry dates
- **Pharmaceuticals**: Export catalog with NDC codes and manufacturer info
- **Transactions**: Export complete transaction history
- **Analytics**: Export dashboard statistics and insights

### QR Code Generation
Generate QR codes for:
- Product identification and tracking
- Batch number verification
- Quick dashboard access
- Inventory item details

### API Status Monitoring
Real-time monitoring of backend API health:
- Endpoint availability status
- Response time tracking
- Average performance metrics
- Grouped by category (Auth, Pharmaceuticals, Inventory, Marketplace)

### Advanced Analytics
Comprehensive business insights:
- Category distribution charts
- Monthly trend analysis
- Top products by value
- Expiry distribution tracking
- Stock utilization metrics
- Low stock and expiry alerts

### Form Validation
Robust validation using Zod:
- Email format validation
- Password strength requirements
- NDC code format validation
- Date range validation
- Numeric constraints
- Custom error messages

## 🔌 API Integration

### Base Configuration
The frontend communicates with the Rust backend API via axios:

```typescript
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
```

### Authentication Flow
1. User logs in via `/api/auth/login`
2. JWT token received and stored in localStorage
3. Token automatically attached to all subsequent requests
4. Token refresh on expiry
5. Automatic logout on invalid token

### Service Layer
All API calls go through dedicated service classes:
- `AuthService`: User authentication and profile management
- `PharmaceuticalService`: Product catalog operations
- `InventoryService`: Stock management
- `MarketplaceService`: Trading and transactions

## 🎨 Customization

### Theme
The app uses Tailwind CSS with custom configuration. Modify colors in `tailwind.config.ts`:

```typescript
theme: {
  extend: {
    colors: {
      // Add your custom colors
    }
  }
}
```

### Components
All UI components are from shadcn/ui and can be customized in `src/components/ui/`

## 🧪 Testing

### Run Type Checking
```bash
npm run type-check
```

### Run Linting
```bash
npm run lint
```

## 🚀 Production Deployment

### Build for Production
```bash
npm run build
```

### Start Production Server
```bash
npm start
```

### Environment Variables for Production
```env
NEXT_PUBLIC_API_URL=https://your-api-domain.com
NEXT_PUBLIC_APP_NAME="Atlas PharmaTech"
NODE_ENV=production
```

### Deployment Platforms
- **Vercel**: Recommended (zero-config)
- **Netlify**: Full support
- **Docker**: Dockerfile ready
- **AWS/GCP/Azure**: Manual deployment supported

## 🐛 Troubleshooting

### API Connection Issues
1. Ensure backend is running on `http://localhost:8080`
2. Check `.env.local` for correct `NEXT_PUBLIC_API_URL`
3. Verify CORS is enabled on backend
4. Check browser console for errors

### Authentication Problems
1. Clear browser localStorage: `localStorage.clear()`
2. Check JWT token expiry
3. Verify backend JWT_SECRET matches
4. Restart both frontend and backend

### Build Errors
1. Delete `.next` folder: `rm -rf .next`
2. Delete `node_modules`: `rm -rf node_modules`
3. Reinstall dependencies: `npm install`
4. Rebuild: `npm run build`

### Port Already in Use
```bash
# Kill process on port 3000
lsof -ti:3000 | xargs kill -9

# Or use a different port
PORT=3001 npm run dev
```

## 📝 Available Scripts

```bash
npm run dev          # Start development server
npm run build        # Build for production
npm start            # Start production server
npm run lint         # Run ESLint
npm run type-check   # Run TypeScript type checking
```

## 🔒 Security Features

- **JWT Authentication**: Secure token-based authentication
- **HTTP-only Cookies**: Option for secure token storage
- **HTTPS Ready**: SSL/TLS support
- **Input Validation**: Zod schema validation on all forms
- **XSS Protection**: Sanitized inputs and outputs
- **CSRF Protection**: Built-in Next.js protection

## 📦 Key Dependencies

```json
{
  "next": "^15.0.3",
  "react": "^18.0.0",
  "typescript": "^5.0.0",
  "tailwindcss": "^3.4.0",
  "axios": "^1.6.0",
  "zod": "^3.22.0",
  "recharts": "^2.10.0",
  "qrcode": "^1.5.3",
  "papaparse": "^5.4.1",
  "xlsx": "^0.18.5",
  "lucide-react": "^0.294.0",
  "react-toastify": "^9.1.3"
}
```

## 🤝 Contributing

This is a production application. For questions or support:
1. Check the troubleshooting section
2. Review existing issues
3. Contact the development team

## 📄 License

Copyright © 2024 Atlas PharmaTech. All rights reserved.

## 🆘 Support

For support and questions:
- Email: support@atlaspharma.com
- Documentation: Internal wiki
- Issues: GitHub Issues

---

**Built with ❤️ using Next.js, TypeScript, and modern web technologies**
