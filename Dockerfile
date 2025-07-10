# ---- Build Stage ----
FROM rust:1.88.0 as builder

WORKDIR /app

# Copy source code and Cargo files
COPY . .

# Build in release mode
RUN cargo build --release

# ---- Final Stage ----
FROM debian:bullseye-slim

# Install necessary system dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /app/target/release/dp-auth-service /app/dp-auth-service

# Set default port if not provided
ENV PORT=3000

# Expose port from environment variable
EXPOSE $PORT

# Run the server
CMD ["./dp-auth-service"]
