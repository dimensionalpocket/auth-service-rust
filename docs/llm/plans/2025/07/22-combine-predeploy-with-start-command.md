# Plan: Update railway.toml to Run preDeployCommand as Part of startCommand

**Date**: 2025-07-22@10:08  
**Task**: Combine the preDeployCommand with startCommand in railway.toml and create a shell script that handles signal propagation for graceful shutdown.

## Current State Analysis

### Current railway.toml Configuration
```toml
[deploy]
startCommand = "./dp-auth-service"
preDeployCommand = "./migrate_and_dump"
healthcheckPath = "/health"
healthcheckTimeout = 30
sleepApplication = false
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 10
numReplicas = 1
```

### Current Docker Setup
- The Dockerfile builds both `dp-auth-service` and `migrate_and_dump` binaries
- Both binaries are copied to `/app/` in the final container
- The container runs in a Debian-based environment
- Database configuration is copied to `/app/config/database`

### Current Graceful Shutdown Implementation
- The `dp-auth-service` has proper signal handling via `ShutdownService`
- Handles both SIGINT and SIGTERM signals
- Uses `axum::serve().with_graceful_shutdown()` for proper connection handling
- Logs which signal was received and shutdown progress

## Problem Statement

Railway's preDeployCommand has a critical limitation with volume-mounted databases:

### The Volume Mounting Issue
- The `migrate_and_dump` script updates the SQLite database file in `/app/data/`
- Railway runs the database on a persistent volume mounted at `/app/data/`
- **Railway's preDeployCommand does NOT mount the volume** during execution
- This means migrations run against an ephemeral SQLite file in the container's filesystem
- When the app starts, the volume mount takes over `/app/data/`, hiding the migrated database
- The result: migrations appear to run successfully but changes are lost

### Additional Requirements
1. Run database migrations (`migrate_and_dump`) with access to the mounted volume
2. Ensure the main service (`dp-auth-service`) receives signals for graceful shutdown
3. Handle the case where the migration might fail and prevent service startup

### Why This Happens
Railway's preDeployCommand behavior is expected - it runs in a separate container without volume mounts to avoid conflicts. This is standard container orchestration behavior, but it means any operations that need persistent storage must run during the main container startup.

## Proposed Solution

### 1. Create a Startup Script with Signal Propagation

Create a new shell script `start.sh` that:
- Runs `migrate_and_dump` first
- If migration succeeds, starts `dp-auth-service`
- Properly forwards SIGINT and SIGTERM signals to the running `dp-auth-service` process
- Exits with the same exit code as `dp-auth-service`

### 2. Update railway.toml

Remove `preDeployCommand` and update `startCommand` to use the new script.

### 3. Update Dockerfile

Make the startup script executable and set it as the default command.

## Implementation Details

### File: `start.sh` (with executable permissions)
**Note**: This file will be created with executable permissions (`chmod +x start.sh`) so it works both locally and when copied to the Docker container.
```bash
#!/bin/bash

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
./migrate_and_dump
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
```

### File: `railway.toml` (Updated)
```toml
[build]
builder = "dockerfile"
# watchPatterns = ["!**/*.md"]

[deploy]
startCommand = "./start.sh"
healthcheckPath = "/health"
healthcheckTimeout = 30
sleepApplication = false
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 10
numReplicas = 1
```

### File: `Dockerfile` (Updated)
```dockerfile
# ---- Build Stage ----
FROM rust:1.88.0-bullseye as builder

WORKDIR /app

# Copy source code and Cargo files
COPY . .

# Build all binaries in release mode
RUN cargo build --release

# ---- Final Stage ----
FROM debian:bullseye-slim

# Install necessary system dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy both compiled binaries from builder
COPY --from=builder /app/target/release/dp-auth-service /app/dp-auth-service
COPY --from=builder /app/target/release/migrate_and_dump /app/migrate_and_dump

# Copy database configuration (migrations, seeds, schema)
COPY --from=builder /app/config/database /app/config/database

# Copy startup script (will be made executable before copying)
COPY start.sh /app/start.sh

RUN mkdir -p data

# Set default port if not provided
ENV PORT=3000

# Expose port from environment variable
EXPOSE $PORT

# Run the startup script
CMD ["./start.sh"]
```

## Signal Handling Flow

1. **Normal Operation**:
   - `start.sh` runs `migrate_and_dump`
   - If successful, starts `dp-auth-service` in background
   - Waits for service to complete

2. **Signal Received (SIGTERM/SIGINT)**:
   - `start.sh` receives signal via trap
   - Logs which signal was received (SIGTERM or SIGINT)
   - Forwards the same signal to `dp-auth-service` process
   - `dp-auth-service` handles signal via `ShutdownService`
   - `dp-auth-service` performs graceful shutdown
   - `start.sh` waits for service to exit and propagates exit code

3. **Migration Failure**:
   - `migrate_and_dump` exits with non-zero code
   - `start.sh` immediately exits with code 1
   - Service never starts, preventing broken state

## Benefits

1. **Volume Access**: Migrations now run with proper access to Railway's mounted volume
2. **Data Persistence**: Database changes are applied to the persistent volume, not ephemeral filesystem
3. **Proper Signal Handling**: Signals are correctly forwarded to the service for graceful shutdown
4. **Fail-Fast**: Migration failures prevent service startup
5. **Container Compatibility**: Works properly in Docker/Railway environment
6. **Exit Code Propagation**: Service exit codes are properly propagated
7. **Logging**: Clear indication of migration success/failure and service status
8. **Railway Compliance**: Works within Railway's expected container orchestration patterns

## Files to Create/Modify

### New Files:
- `start.sh` - Startup script with signal handling (created with executable permissions)

### Modified Files:
- `railway.toml` - Remove preDeployCommand, update startCommand
- `Dockerfile` - Copy startup script, update CMD

## Testing Strategy

1. **Local Testing**:
   - Test script with successful migration
   - Test script with failed migration
   - Test signal forwarding (SIGTERM/SIGINT)
   - Verify exit code propagation

2. **Container Testing**:
   - Build Docker image and test locally
   - Test signal handling in container environment
   - Verify Railway deployment works correctly

3. **Integration Testing**:
   - Deploy to Railway staging environment
   - Test health checks work correctly
   - Test restart behavior on failure
   - Verify graceful shutdown in Railway environment

## Rollback Plan

If issues arise:
1. Revert `railway.toml` to use separate preDeployCommand and startCommand
2. Revert `Dockerfile` CMD to `["./dp-auth-service"]`
3. Remove `start.sh` script

The original configuration can be restored quickly since no core application logic is changed.