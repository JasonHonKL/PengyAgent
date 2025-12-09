# Multi-stage Dockerfile for PengyAgent
# Stage 1: Build stage
FROM rust:latest as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the release binary
# Build both binaries, but we'll primarily use pengy-cmd for Docker
RUN cargo build --release --bin pengy-cmd && \
    cargo build --release --bin pengy

# Stage 2: Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /app/target/release/pengy-cmd /usr/local/bin/pengy-cmd
COPY --from=builder /app/target/release/pengy /usr/local/bin/pengy

# Set pengy-cmd as the default entrypoint (non-interactive mode)
ENTRYPOINT ["pengy-cmd"]

