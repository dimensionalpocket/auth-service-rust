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
COPY --from=builder /app/target/release/run_local_server /app/dps-auth-api
COPY --from=builder /app/target/release/dps-auth-api-migrate /app/dps-auth-api-migrate

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
