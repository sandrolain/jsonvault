# Multi-stage build for JsonVault
FROM rust:1.87-slim AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/
COPY benches/ ./benches/
COPY examples/ ./examples/

# Build the application in release mode
RUN cargo build --release --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies and create user
RUN apt-get update && apt-get install -y \
  ca-certificates \
  curl \
  && rm -rf /var/lib/apt/lists/* \
  && groupadd -r jsonvault \
  && useradd -r -g jsonvault jsonvault

# Create app directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/server /usr/local/bin/jsonvault-server

# Create data directory
RUN mkdir -p /data && chown jsonvault:jsonvault /data

# Switch to non-root user
USER jsonvault

# Expose default port
EXPOSE 8080

# Set default environment variables
ENV RUST_LOG=info
ENV JSONVAULT_PORT=8080
ENV JSONVAULT_DATA_DIR=/data
ENV JSONVAULT_NODE_ID=1

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:${JSONVAULT_PORT}/health || exit 1

# Default command with Raft consensus
CMD ["sh", "-c", "jsonvault-server --enable-raft --address 0.0.0.0:${JSONVAULT_PORT} --node-id ${JSONVAULT_NODE_ID}"]
