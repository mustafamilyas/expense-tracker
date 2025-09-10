#!/usr/bin/env bash
set -euo pipefail

# Usage: DATABASE_URL=postgres://... bash bin/seed.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR}/.."

cd "$REPO_ROOT"

echo "Building seed binary..."
cargo build --bin seed

echo "Running seeder..."
cargo run --quiet --bin seed

echo "Done."

