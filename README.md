# Expense Tracker

An expense tracking application with Telegram integration.

## Features

- Track expenses by category and group
- Web dashboard for expense management
- Telegram bot integration for chat-based expense tracking
- Modular messenger system (easily extensible to WhatsApp, etc.)

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and fill in the required values:
   ```bash
   cp .env.example .env
   ```

3. Set up the database:
   ```bash
   # Make sure PostgreSQL is running
   createdb expense_tracker
   cargo run --bin seed
   ```

4. Run the application:
   ```bash
   cargo run
   ```

## Telegram Integration

### Setup

1. Create a Telegram bot:
   - Message @BotFather on Telegram
   - Use `/newbot` command
   - Follow the instructions to create your bot
   - Copy the bot token

2. Add the bot token to your `.env` file:
   ```
   TELEGRAM_BOT_TOKEN=your-bot-token-here
   ```

3. Restart the application

### Usage

1. Start a chat with your bot on Telegram
2. Send `/sign-in` to initiate the binding process
3. The bot will provide a URL to bind the chat to an expense group
4. Once bound, any message sent to the bot will be echoed back

### Architecture

The messenger system is designed to be modular:

- `src/messengers/mod.rs`: Common traits and interfaces
- `src/messengers/telegram.rs`: Telegram-specific implementation
- Easy to add new messengers (WhatsApp, Discord, etc.) by implementing the `Messenger` trait

## API Documentation

The API documentation is available at `/swagger-ui` when the server is running.

## Development

### VSCode Debugging

The project includes VSCode debugger configurations for both the Rust backend and web frontend:

#### Prerequisites
- Install recommended VSCode extensions (see `.vscode/extensions.json`)
- For Rust debugging: Install CodeLLDB extension
- For web debugging: Install Chrome debugger extension

#### Debug Configurations
1. **Debug Rust Backend**: Launches the expense tracker server with debugging
2. **Debug Rust Tests**: Runs tests with debugger attached
3. **Debug Web App**: Launches Vite dev server with Node.js debugging
4. **Debug Web App (Chrome)**: Launches Chrome with source maps for frontend debugging

#### Environment Setup
The debugger configurations automatically set up required environment variables:
- `DATABASE_URL`: PostgreSQL connection string
- `JWT_SECRET`: Development JWT secret
- `CHAT_RELAY_SECRET`: Development chat relay secret

Make sure PostgreSQL is running before debugging the Rust application.

### Adding a New Messenger

1. Create a new module in `src/messengers/`
2. Implement the `Messenger` trait
3. Add the messenger to the `MessengerManager` in `main.rs`

Example:
```rust
// In src/messengers/whatsapp.rs
pub struct WhatsAppMessenger { ... }

#[async_trait]
impl Messenger for WhatsAppMessenger {
    // Implement required methods
}
```

## CI/CD

The project uses GitHub Actions for continuous integration with code coverage reporting:

- **Main branch**: Runs all tests and builds with coverage
- **Development branch**: Runs selective tests based on changed files with coverage
- **Coverage**: Reports uploaded to Codecov, visible on pull requests

See [docs/ci.md](docs/ci.md) for detailed CI documentation.

## Environment Variables

- `JWT_SECRET`: Secret key for JWT token generation
- `CHAT_RELAY_SECRET`: Secret for webhook verification
- `TELEGRAM_BOT_TOKEN`: Token for Telegram bot (optional)
- `TELEGRAM_LOG_BOT_TOKEN`: Separate bot token for logging (optional)
- `TELEGRAM_LOG_CHAT_ID`: Chat ID for logging messages (optional)
- `DATABASE_URL`: PostgreSQL connection string