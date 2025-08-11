#!/bin/bash

# This script relies on both binaries (dp-auth-migrate and dp-auth-service) being in the root path.

# Exit on any error
set -e

# Function to handle signals and forward them to the service
cleanup() {
    local signal=$1
    echo "[Start] Received $signal signal, forwarding to dp-auth-service..."
    if [ ! -z "$SERVICE_PID" ]; then
        # Forward the same signal to the service process
        kill -$signal "$SERVICE_PID" 2>/dev/null || true
        # Wait for the service to exit gracefully
        wait "$SERVICE_PID" 2>/dev/null || true
    fi
    exit 0
}

# Set up signal handlers with specific signal names
trap 'cleanup TERM' SIGTERM
trap 'cleanup INT' SIGINT

echo "[Start] Starting database migration, seeding, and schema dump..."

# Run migrations first - exit if this fails
./dp-auth-migrate
if [ $? -ne 0 ]; then
    echo "[Start] ❌ Migration failed, aborting startup"
    exit 1
fi

echo "[Start] ✅ Migration completed successfully, starting service..."

# Start the main service in the background
./dp-auth-service &
SERVICE_PID=$!

# Wait for the service to complete
wait "$SERVICE_PID"
SERVICE_EXIT_CODE=$?

echo "[Start] Service exited with code: $SERVICE_EXIT_CODE"
exit $SERVICE_EXIT_CODE
