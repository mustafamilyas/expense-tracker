# Expense Tracker Documentation

A comprehensive expense tracking application with Telegram bot integration and subscription-based tier system.

## 📋 Table of Contents

- [Features](#features)
- [Architecture](#architecture)
- [Codebase Structure](#codebase-structure)
- [Database Schema](#database-schema)
- [API Documentation](#api-documentation)
- [Telegram Bot](#telegram-bot)
- [Subscription Tiers](#subscription-tiers)
- [Setup & Installation](#setup--installation)
- [Development](#development)
- [Testing](#testing)
- [Next Steps](#next-steps)

## ✨ Features

### Core Functionality

- **Expense Tracking**: Track expenses by category and group with detailed metadata
- **Group Management**: Create and manage expense groups for shared tracking
- **Category System**: Organize expenses with customizable categories and aliases
- **Budget Management**: Set monthly budgets per category with spending alerts
- **Real-time Reports**: Generate monthly expense reports with charts and analytics

### Integration Features

- **Telegram Bot**: Full-featured bot for expense tracking via chat
- **Web Dashboard**: Modern React-based interface for expense management
- **REST API**: Complete REST API for third-party integrations
- **Automated Reports**: Scheduled PDF report generation and delivery

### Advanced Features

- **Subscription Tiers**: Multi-tier subscription system with usage limits
- **Usage Analytics**: Real-time usage tracking and analytics
- **Data Export**: Export capabilities for data portability
- **Multi-user Support**: Group-based collaboration with access controls

## 🏗️ Architecture

### System Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Frontend  │    │   Telegram Bot  │    │   REST API      │
│   (React)       │    │   (Rust)        │    │   (Rust)        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   Core Engine  │
                    │   (Rust)       │
                    │                │
                    │ • Business Logic│
                    │ • Data Access  │
                    │ • Tier System  │
                    │ • Report Gen   │
                    └─────────────────┘
                             │
                    ┌─────────────────┐
                    │   PostgreSQL   │
                    │   Database     │
                    └─────────────────┘
```

### Technology Stack

- **Backend**: Rust with Axum web framework
- **Database**: PostgreSQL with SQLx ORM
- **Frontend**: React with TypeScript, Tailwind CSS, Vite
- **Bot**: Telegram Bot API integration
- **Reports**: PDF generation with custom charting
- **Scheduling**: Cron-based background jobs

## 📁 Codebase Structure

### Backend (Rust)

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library exports
├── app.rs                  # Axum application setup
├── auth.rs                 # Authentication middleware
├── db.rs                   # Database connection utilities
├── error.rs                # Error handling types
├── types.rs                # Shared types and enums
├── messengers/             # Communication integrations
│   ├── mod.rs
│   ├── telegram.rs         # Telegram bot implementation
│   └── messenger.rs        # Messenger trait definitions
├── middleware/             # Axum middleware
│   ├── mod.rs
│   ├── auth.rs             # Authentication middleware
│   └── tier.rs             # Subscription tier enforcement
├── repos/                  # Data access layer
│   ├── mod.rs
│   ├── user.rs             # User repository
│   ├── expense_entry.rs    # Expense entry repository
│   ├── expense_group.rs    # Expense group repository
│   ├── category.rs         # Category repository
│   ├── category_alias.rs   # Category alias repository
│   ├── budget.rs           # Budget repository
│   ├── chat_binding.rs     # Chat binding repository
│   ├── chat_bind_request.rs # Chat bind request repository
│   └── subscription.rs     # Subscription repository
├── routes/                 # API route handlers
│   ├── mod.rs
│   ├── users.rs            # User management routes
│   ├── expense_entries.rs  # Expense entry routes
│   ├── expense_groups.rs   # Expense group routes
│   ├── categories.rs       # Category routes
│   ├── budgets.rs          # Budget routes
│   ├── chat_bindings.rs    # Chat binding routes
│   ├── health.rs           # Health check routes
│   └── version.rs          # Version info routes
├── reports/                # Report generation
│   ├── mod.rs
│   ├── monthly_report.rs   # Monthly report generator
│   └── scheduler.rs        # Report scheduling
└── openapi.rs              # OpenAPI documentation
```

### Frontend (React/TypeScript)

```
apps/web/
├── src/
│   ├── components/         # Reusable UI components
│   │   ├── Guard.tsx       # Authentication guard
│   │   └── ...
│   ├── lib/                # Utilities and API client
│   │   ├── api.ts          # API client functions
│   │   └── auth.ts         # Authentication utilities
│   ├── routes/             # Page components
│   │   ├── Dashboard.tsx   # Main dashboard
│   │   ├── Register.tsx    # Registration page
│   │   ├── SignIn.tsx      # Sign-in page
│   │   └── ChatBindConfirm.tsx # Chat binding confirmation
│   ├── App.tsx             # Main application component
│   ├── main.tsx            # Application entry point
│   └── index.css           # Global styles
├── index.html              # HTML template
├── package.json            # Dependencies and scripts
├── tailwind.config.js      # Tailwind CSS configuration
├── postcss.config.cjs      # PostCSS configuration
├── tsconfig.json           # TypeScript configuration
└── vite.config.ts          # Vite configuration
```

### Database Migrations

```
migrations/
├── 20250120000000_add_subscriptions.up.sql    # Subscription system
├── 20250120000000_add_subscriptions.down.sql
├── 20250907132444_init.up.sql                 # Initial schema
└── 20250907132444_init.down.sql
```

## 🗄️ Database Schema

### Core Tables

#### Users
```sql
CREATE TABLE users (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    phash VARCHAR(255) NOT NULL,
    start_over_date SMALLINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### Expense Groups
```sql
CREATE TABLE expense_groups (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    owner UUID NOT NULL REFERENCES users(uid) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### Group Members
```sql
CREATE TABLE group_members (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_uid UUID NOT NULL REFERENCES expense_groups(uid) ON DELETE CASCADE,
    user_uid UUID NOT NULL REFERENCES users(uid) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(group_uid, user_uid)
);
```

#### Categories
```sql
CREATE TABLE categories (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_uid UUID NOT NULL REFERENCES expense_groups(uid) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### Expense Entries
```sql
CREATE TABLE expense_entries (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_uid UUID NOT NULL REFERENCES expense_groups(uid) ON DELETE CASCADE,
    category_uid UUID NOT NULL REFERENCES categories(uid) ON DELETE CASCADE,
    price DECIMAL(15,2) NOT NULL,
    product VARCHAR(500) NOT NULL,
    created_by UUID NOT NULL REFERENCES users(uid) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

### Subscription System

#### Subscriptions
```sql
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid) ON DELETE CASCADE,
    tier subscription_tier NOT NULL DEFAULT 'free',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    current_period_start TIMESTAMP WITH TIME ZONE,
    current_period_end TIMESTAMP WITH TIME ZONE,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

#### User Usage Tracking
```sql
CREATE TABLE user_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid) ON DELETE CASCADE,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    groups_count INTEGER NOT NULL DEFAULT 0,
    total_expenses INTEGER NOT NULL DEFAULT 0,
    total_members INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_uid, period_start, period_end)
);
```

## 🔌 API Documentation

### Authentication

All API endpoints require Bearer token authentication:

```
Authorization: Bearer <jwt_token>
```

### Core Endpoints

#### Users
- `POST /users` - Create user account
- `GET /users/me` - Get current user profile
- `PUT /users/me` - Update user profile

#### Expense Groups
- `GET /expense-groups` - List user's groups
- `POST /expense-groups` - Create new group
- `GET /expense-groups/{uid}` - Get group details
- `PUT /expense-groups/{uid}` - Update group
- `DELETE /expense-groups/{uid}` - Delete group

#### Expense Entries
- `POST /expense-entries` - Create expense entry
- `GET /groups/{group_uid}/expense-entries` - List group expenses
- `GET /expense-entries/{uid}` - Get expense details
- `PUT /expense-entries/{uid}` - Update expense
- `DELETE /expense-entries/{uid}` - Delete expense

#### Categories
- `GET /groups/{group_uid}/categories` - List group categories
- `POST /categories` - Create category
- `GET /categories/{uid}` - Get category details
- `PUT /categories/{uid}` - Update category
- `DELETE /categories/{uid}` - Delete category

#### Budgets
- `GET /budgets/group/{group_uid}` - List group budgets
- `POST /budgets` - Create budget
- `GET /budgets/{uid}` - Get budget details
- `PUT /budgets/{uid}` - Update budget
- `DELETE /budgets/{uid}` - Delete budget

### OpenAPI Specification

The API is fully documented with OpenAPI 3.0. Access the interactive documentation at:

```
/swagger-ui
```

## 🤖 Telegram Bot

### Setup

1. Create a bot with [@BotFather](https://t.me/botfather)
2. Add the bot token to your `.env` file:
   ```
   TELEGRAM_BOT_TOKEN=your-bot-token-here
   ```
3. Start the application - the bot will be automatically initialized

### Available Commands

#### Basic Commands
- `/sign-in` - Initiate chat binding process
- `/command` - Show all available commands
- `/subscription` - View subscription status and usage

#### Expense Management
- `/expense [product],[price],[category]` - Add new expense
- `/expense-edit [id] [product],[price],[category]` - Edit existing expense
- `/report` - View monthly expense summary
- `/history` - View detailed expense history

#### Category Management
- `/category` - List all categories and aliases
- `/category-add [name]` - Add new category
- `/category-edit [old_name] [new_name]` - Rename category
- `/category-alias [alias] [category_name]` - Add category alias

#### Budget Management
- `/budget` - View budget overview
- `/budget-add [category] [amount]` - Add budget for category
- `/budget-edit [category] [new_amount]` - Update budget
- `/budget-remove [category]` - Remove budget

#### Advanced Features
- `/generate-report` - Generate monthly PDF report
- `/budget` - View budget overview with spending alerts

### Usage Examples

```
# Add expense with category
/expense Coffee,15000,Food & Beverage

# Add expense with auto-categorization
/expense Lunch,25000

# Edit expense
/expense-edit abc123 Lunch,30000,Food

# Add category
/category-add Transportation

# Set budget
/budget-add Food & Beverage 500000
```

## 💰 Subscription Tiers

### Tier Comparison

| Feature | Free | Personal | Family | Team | Enterprise |
|---------|------|----------|--------|------|------------|
| **Groups** | 1 | 1 | 3 | 10 | Unlimited |
| **Members per Group** | 1 | 2 | 10 | 50 | Unlimited |
| **Categories per Group** | 5 | 20 | 50 | 100 | Unlimited |
| **Budgets per Group** | 3 | 10 | 25 | 50 | Unlimited |
| **Monthly Expenses** | 100 | 1,000 | 5,000 | 25,000 | Unlimited |
| **Data Retention** | 90 days | 1 year | 1 year | 2 years | 7 years |
| **Advanced Reports** | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Data Export** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Priority Support** | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Custom Categories** | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Price** | $0 | $4.99 | $9.99 | $19.99 | $49.99 |

### Usage Tracking

The system automatically tracks:
- Number of active groups
- Total group members
- Monthly expense count
- Categories and budgets per group

### Upgrade Prompts

Users receive helpful upgrade suggestions when:
- Approaching 80% of any limit
- Attempting to exceed tier limits
- Using premium features on free tier

## 🚀 Setup & Installation

### Prerequisites

- Rust 1.70+
- PostgreSQL 13+
- Node.js 18+ (for frontend development)

### Backend Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd expense-tracker
   ```

2. **Environment Configuration**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. **Database Setup**
   ```bash
   # Create PostgreSQL database
   createdb expense_tracker

   # Run database migrations
   cargo run --bin seed
   ```

4. **Run the Application**
   ```bash
   cargo run
   ```

### Frontend Setup

1. **Navigate to frontend directory**
   ```bash
   cd apps/web
   ```

2. **Install dependencies**
   ```bash
   yarn install
   ```

3. **Start development server**
   ```bash
   yarn dev
   ```

### Docker Setup (Alternative)

```bash
# Build and run with Docker Compose
docker-compose up --build
```

## 💻 Development

### Code Organization

#### Repository Pattern
All data access is abstracted through repository structs:
- Clean separation of concerns
- Easy to test and mock
- Consistent error handling

#### Middleware System
- Authentication middleware for protected routes
- Tier enforcement middleware for subscription limits
- Extensible for additional cross-cutting concerns

#### Error Handling
- Custom error types for different domains
- Consistent error responses across API
- Proper error propagation and logging

### Development Workflow

1. **Feature Development**
   - Create feature branch from `main`
   - Implement changes with tests
   - Update documentation as needed

2. **Testing**
   ```bash
   # Run all tests
   cargo test

   # Run specific test
   cargo test test_name

   # Run with coverage
   cargo tarpaulin
   ```

3. **Code Quality**
   ```bash
   # Format code
   cargo fmt

   # Lint code
   cargo clippy

   # Check compilation
   cargo check
   ```

### Adding New Features

#### New API Endpoint
1. Add repository method in `src/repos/`
2. Add route handler in `src/routes/`
3. Add tier checks if needed
4. Update OpenAPI documentation
5. Add tests

#### New Telegram Command
1. Add command handler in `src/messengers/telegram.rs`
2. Add tier checks if needed
3. Update command list in `/command` handler
4. Add tests

#### New Subscription Feature
1. Update `TierLimits` in `src/types.rs`
2. Add enforcement in relevant middleware
3. Update API routes and Telegram commands
4. Add tests for limit enforcement

## 🧪 Testing

### Test Structure

```
tests/
└── repos_tests.rs    # Repository and business logic tests
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test tier_limits_enforcement_test

# Run with detailed output
cargo test -- --nocapture

# Run tests with database
DATABASE_URL=postgresql://localhost/expense_tracker_test cargo test
```

### Test Coverage

Current test coverage includes:
- ✅ Repository CRUD operations
- ✅ Subscription tier limit enforcement
- ✅ Business logic validation
- ✅ Error handling scenarios

## 🎯 Next Steps

### Immediate Priorities

#### 1. Stripe Integration
- Implement payment processing for subscription upgrades
- Handle webhook events for subscription status changes
- Add billing management interface

#### 2. Enhanced Analytics
- Real-time usage dashboard in web interface
- Advanced reporting with charts and trends
- Export functionality for data portability

#### 3. User Experience Improvements
- Email notifications for important events
- Improved onboarding flow
- Mobile-responsive web interface

### Medium-term Goals

#### 4. Advanced Features
- Recurring expenses and subscriptions
- Expense approval workflows for teams
- Integration with banking APIs
- Multi-currency support

#### 5. Performance Optimization
- Database query optimization
- Caching layer for frequently accessed data
- Horizontal scaling considerations

#### 6. Security Enhancements
- Rate limiting for API endpoints
- Audit logging for sensitive operations
- Data encryption at rest

### Long-term Vision

#### 7. Enterprise Features
- SSO integration (SAML, OAuth)
- Advanced permission system
- Custom integrations and webhooks
- White-label solutions

#### 8. AI/ML Integration
- Expense categorization using ML
- Anomaly detection for unusual spending
- Predictive budgeting and forecasting

#### 9. Mobile Applications
- Native iOS and Android apps
- Offline expense tracking
- Receipt scanning with OCR

### Technical Debt & Improvements

#### 10. Code Quality
- Add integration tests for API endpoints
- Implement comprehensive logging and monitoring
- Add performance benchmarks
- Improve error messages and user feedback

#### 11. Infrastructure
- CI/CD pipeline setup
- Container orchestration with Kubernetes
- Database backup and recovery procedures
- Monitoring and alerting system

### Community & Documentation

#### 12. Documentation
- API client SDKs for popular languages
- Video tutorials and getting started guides
- Community forum and support channels
- Contributing guidelines for open source

---

## 📞 Support

For questions, issues, or contributions:

- **GitHub Issues**: [Report bugs and request features](https://github.com/your-repo/issues)
- **Documentation**: [Full API docs and guides](https://docs.your-project.com)
- **Community**: [Join our Discord/Slack community](https://community.your-project.com)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.</content>
</xai:function_call">README.md