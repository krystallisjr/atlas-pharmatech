# Atlas Pharma - Pharmaceutical Inventory Trading Platform

A modern, production-ready pharmaceutical inventory management and marketplace platform built with Rust, Axum, PostgreSQL, and TypeScript.

## Features

- **User Management**: Registration, authentication, and profile management
- **Inventory Management**: Add/update pharmaceutical stock with expiry tracking
- **Marketplace**: Buyer-seller interactions with inquiries and transactions
- **Expiry Alerts**: Automated notifications for approaching expiry dates
- **Compliance**: Full audit trail and batch number tracking
- **Search**: Advanced filtering by brand, generic name, manufacturer, NDC codes

## Tech Stack

- **Backend**: Rust + Axum + SQLx + PostgreSQL
- **Frontend**: TypeScript + React (to be implemented)
- **Database**: PostgreSQL with comprehensive indexing
- **Authentication**: JWT-based with secure password hashing

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL 13+
- Node.js 16+ (for frontend)

### Database Setup

1. Create a PostgreSQL database:
```sql
CREATE DATABASE atlas_pharma;
```

2. Run migrations:
```bash
psql -d atlas_pharma -f migrations/001_initial_schema.sql
```

### Configuration

1. Copy environment variables:
```bash
cp .env.example .env
```

2. Update `.env` with your database credentials and JWT secret.

### Running the Application

1. Install dependencies:
```bash
cargo build
```

2. Run the server:
```bash
cargo run
```

The API will be available at `http://localhost:8080`

## API Endpoints

### Authentication
- `POST /api/auth/register` - Register new user
- `POST /api/auth/login` - User login
- `POST /api/auth/refresh` - Refresh JWT token
- `GET /api/auth/profile` - Get user profile
- `PUT /api/auth/profile` - Update user profile
- `DELETE /api/auth/delete` - Delete user account

### Pharmaceuticals (Verified users only)
- `POST /api/pharmaceuticals` - Add new pharmaceutical
- `GET /api/pharmaceuticals/:id` - Get pharmaceutical details
- `GET /api/pharmaceuticals/search` - Search pharmaceuticals
- `GET /api/pharmaceuticals/manufacturers` - Get all manufacturers
- `GET /api/pharmaceuticals/categories` - Get all categories

### Inventory (Authenticated users)
- `POST /api/inventory` - Add inventory item
- `GET /api/inventory/:id` - Get inventory details
- `GET /api/inventory/my` - Get user's inventory
- `PUT /api/inventory/:id` - Update inventory item
- `DELETE /api/inventory/:id` - Delete inventory item

### Marketplace (Authenticated users)
- `POST /api/marketplace/inquiries` - Create inquiry
- `GET /api/marketplace/inquiries/:id` - Get inquiry details
- `GET /api/marketplace/inquiries/buyer` - Get buyer's inquiries
- `GET /api/marketplace/inquiries/seller` - Get seller's inquiries
- `PUT /api/marketplace/inquiries/:id/status` - Update inquiry status
- `POST /api/marketplace/transactions` - Create transaction
- `GET /api/marketplace/transactions/:id` - Get transaction details
- `GET /api/marketplace/transactions/my` - Get user's transactions
- `POST /api/marketplace/transactions/:id/complete` - Complete transaction
- `POST /api/marketplace/transactions/:id/cancel` - Cancel transaction

### Public Endpoints
- `GET /api/public/inventory/search` - Search marketplace inventory
- `GET /api/public/expiry-alerts` - Get expiry alerts

## Database Schema

The application uses a comprehensive schema with the following main tables:
- `users` - User accounts and company information
- `pharmaceuticals` - Product catalog with NDC codes
- `inventory` - Stock records with expiry dates and batch numbers
- `inquiries` - Buyer inquiries for inventory items
- `transactions` - Transaction records
- `inventory_audit` - Audit trail for compliance

## Development

### Running Tests

```bash
cargo test
```

### Code Structure

- `src/models/` - Data models and validation
- `src/repositories/` - Database operations
- `src/services/` - Business logic layer
- `src/handlers/` - HTTP request handlers
- `src/middleware/` - Authentication and error handling
- `src/config/` - Configuration management

### Database Migrations

Migrations are stored in the `migrations/` directory. To add new migrations:

1. Create a new SQL file with appropriate naming
2. Add your schema changes
3. Update the application code as needed

## License

This project is proprietary software. All rights reserved.