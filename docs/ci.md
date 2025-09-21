# CI/CD Pipeline

This document describes the Continuous Integration setup for the Expense Tracker project.

## Overview

The CI pipeline is configured using GitHub Actions and runs different test suites based on the target branch:

- **Main branch**: Runs all tests and builds for both Rust backend and web frontend
- **Development branch**: Runs selective tests based on what files changed

## Workflow Details

### Triggers
- Push to `main` or `development` branches
- Pull requests targeting `main` or `development`
- Only runs when relevant files change (Rust files, web files, or dependency files)

### Coverage Reporting
- **Rust**: Uses `cargo-tarpaulin` for code coverage
- **Web**: Uses Vitest with v8 coverage provider
- Coverage reports are uploaded to Codecov
- Coverage comments appear on pull requests
- Coverage badges can be added to README

### Jobs

#### Main Branch (`test-main`)
When code is pushed to or merged into `main`:
1. Sets up Rust toolchain and Node.js
2. Installs dependencies for both backend and frontend
3. Runs Rust checks (cargo check, clippy)
4. Runs all Rust tests with PostgreSQL database
5. Builds the web application

#### Development Branch (`test-development`)
When code is pushed to `development`:
1. Analyzes what files changed since the last commit
2. If Rust files changed: runs Rust checks and tests
3. If web files changed: runs web tests and build
4. If no specific changes: runs basic checks only

## Database Setup

The CI uses a PostgreSQL service for running database tests:
- PostgreSQL 15
- Database: `postgres`
- User: `postgres`
- Password: `postgres`

## Adding Tests

### Rust Backend
Tests are located in the `tests/` directory. Add new test files there and they will be automatically picked up by `cargo test`.

### Web Frontend
Currently, the web app has placeholder test scripts. To add real tests:

1. Install a testing framework (e.g., Vitest for Vite projects)
2. Add test scripts to `apps/web/package.json`
3. Update the CI workflow if needed

Example for adding Vitest:
```bash
cd apps/web
yarn add -D vitest @testing-library/jest-dom
```

Then update `package.json`:
```json
{
  "scripts": {
    "test": "vitest",
    "test:ui": "vitest --ui"
  }
}
```

## Environment Variables

The CI expects these environment variables to be set in your repository secrets (if needed):
- `DATABASE_URL`: Set automatically by the PostgreSQL service in CI

## Local Development

To run tests locally:

```bash
# Rust tests
cargo test

# Rust tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Web tests
cd apps/web && yarn test

# Web tests with coverage
cd apps/web && yarn test:coverage

# Web build
cd apps/web && yarn build

# Full test suite with coverage
cargo tarpaulin --out Html && cd apps/web && yarn test:coverage && yarn build
```

## Coverage

### Viewing Coverage Locally
- **Rust**: Open `tarpaulin-report.html` in your browser after running `cargo tarpaulin --out Html`
- **Web**: Open `apps/web/coverage/index.html` in your browser after running `yarn test:coverage`

### Coverage Reports on PRs
- Codecov will comment on pull requests with coverage changes
- Coverage reports show line-by-line coverage
- Failed coverage checks can block merges (configure in repository settings)

### Coverage Thresholds
Consider setting minimum coverage thresholds in your CI:
- Rust: 80% line coverage
- Web: 70% line coverage (when tests are added)

Configure thresholds in your Codecov settings or add checks to the CI workflow.

### Codecov Integration
1. Sign up at [codecov.io](https://codecov.io) and connect your GitHub repository
2. The CI will automatically upload coverage reports
3. Add coverage badges to your README:

```markdown
[![codecov](https://codecov.io/gh/yourusername/expense-tracker/branch/main/graph/badge.svg)](https://codecov.io/gh/yourusername/expense-tracker)
[![codecov](https://codecov.io/gh/yourusername/expense-tracker/branch/main/graph/badge.svg?flag=rust)](https://codecov.io/gh/yourusername/expense-tracker)
[![codecov](https://codecov.io/gh/yourusername/expense-tracker/branch/main/graph/badge.svg?flag=web)](https://codecov.io/gh/yourusername/expense-tracker)
```

### Coverage Checks
To enforce coverage requirements, add status checks in your repository settings:
1. Go to repository Settings → Branches
2. Add rule for `main` branch
3. Require "codecov/patch" and "codecov/project" checks to pass

## Branch Protection

Consider setting up branch protection rules:
- Require CI to pass before merging to `main`
- Require code review for changes to `main`
- Allow direct pushes to `development` for faster iteration