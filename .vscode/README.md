# VSCode Debugging Setup

This directory contains VSCode configuration files for debugging both the Rust backend and web frontend applications.

## Prerequisites

### Required VSCode Extensions
Install the following extensions (automatically recommended when opening the workspace):

- **Rust Analyzer** (`rust-lang.rust-analyzer`): Rust language support and IntelliSense
- **CodeLLDB** (`vadimcn.vscode-lldb`): Rust debugging support
- **TypeScript Importer** (`pmneo.tsimporter`): Auto import for TypeScript
- **Prettier** (`esbenp.prettier-vscode`): Code formatting
- **Tailwind CSS IntelliSense** (`bradlc.vscode-tailwindcss`): Tailwind CSS support
- **Chrome Debugger** (`msjsdiag.debugger-for-chrome`): Frontend debugging

### System Requirements
- **PostgreSQL**: Running locally for database operations
- **Node.js** and **Yarn**: For web application development
- **Rust**: Latest stable version with Cargo

## Debug Configurations

### 1. Debug Rust Backend
- **Purpose**: Debug the main expense tracker server
- **Configuration**: Launches the compiled binary with LLDB debugger
- **Environment**: Sets up database connection and secrets
- **Pre-launch**: Builds the project with `cargo build`

### 2. Debug Rust Tests
- **Purpose**: Debug unit and integration tests
- **Configuration**: Runs tests with debugger attached
- **Pre-launch**: Compiles tests with `cargo test --no-run`

### 3. Debug Web App
- **Purpose**: Debug the Vite development server
- **Configuration**: Launches Vite with Node.js debugger
- **Port**: Runs on default Vite port (5173)

### 4. Debug Web App (Chrome)
- **Purpose**: Debug frontend code in Chrome browser
- **Configuration**: Launches Chrome with source map support
- **URL**: Opens http://localhost:5173

## Getting Started

1. **Open in VSCode**: Open the project root folder
2. **Install Extensions**: VSCode will prompt to install recommended extensions
3. **Set Environment**: Ensure PostgreSQL is running and `.env` file exists
4. **Start Debugging**: Use F5 or go to Run → Start Debugging
5. **Select Configuration**: Choose the appropriate debug configuration

## Troubleshooting

### Rust Debugging Issues
- Ensure CodeLLDB extension is installed and up to date
- Check that `cargo build` completes successfully
- Verify PostgreSQL connection string in environment variables

### Web Debugging Issues
- Ensure dependencies are installed: `cd apps/web && yarn install`
- Check that port 5173 is available
- For Chrome debugging, ensure Chrome is installed

### Common Errors
- **"Cannot find module"**: Run `yarn install` in the web directory
- **"Connection refused"**: Ensure PostgreSQL is running
- **"Debug adapter not found"**: Install required VSCode extensions

## Environment Variables

The debug configurations automatically set development environment variables. For production debugging, update the `.env` file with appropriate values.

## Additional Tasks

The `tasks.json` file includes additional build tasks:
- `cargo check`: Fast compilation checking
- `yarn install`: Install web dependencies
- `yarn dev`: Start development server without debugging