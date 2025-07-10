# Multi-stage build for minimal image size and low memory usage

# Build stage - use specific Rust version from .tool-versions
FROM rust:1.88.0-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /app

# Copy dependency files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached if Cargo.toml doesn't change)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the application
# Touch main.rs to ensure it's rebuilt
RUN touch src/main.rs && cargo build --release

# Runtime stage - use minimal Alpine image
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Create non-root user for security
RUN addgroup -g 1000 appuser && \
    adduser -D -s /bin/sh -u 1000 -G appuser appuser

# Create data directory for SQLite database
RUN mkdir -p /app/data && chown -R appuser:appuser /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/dp-auth-service /usr/local/bin/dp-auth-service

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /app

# Set default port if not provided
ENV PORT=3000

# Expose port from environment variable
EXPOSE $PORT


# Run the application
CMD ["dp-auth-service"]