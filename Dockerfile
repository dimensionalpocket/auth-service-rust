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

# Set default port if not provided
ENV PORT=3000

# Expose port from environment variable
EXPOSE $PORT

# Run the server
CMD ["./dp-auth-service"]
