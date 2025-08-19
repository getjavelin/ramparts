# Multi-stage build for optimal image size
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY scan/Cargo.toml ./scan/
COPY proxy/Cargo.toml ./proxy/
COPY common/Cargo.toml ./common/

# Copy source code
COPY scan/src ./scan/src/
COPY proxy/src ./proxy/src/
COPY common/src ./common/src/
COPY scan/build.rs ./scan/

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false ramparts

# Copy binary from builder stage
COPY --from=builder /app/target/release/ramparts /usr/local/bin/ramparts

# Set ownership and permissions
RUN chown ramparts:ramparts /usr/local/bin/ramparts

# Switch to non-root user
USER ramparts

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
ENTRYPOINT ["ramparts"]
CMD ["proxy", "0.0.0.0:8080"]
