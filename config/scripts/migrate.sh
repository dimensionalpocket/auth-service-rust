#!/bin/bash

# Change to project root directory
cd "$(dirname "$0")/../.."

# Run the migration script (dotenvy handles .env loading)
echo "Running database migrations, seeds, and schema dump..."
mise exec -- cargo run --bin dps-auth-api-migrate

echo "Migration script completed."