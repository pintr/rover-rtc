# Multi-stage build for efficient Docker image
FROM rust:1.75-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    iproute2 \
    iputils-ping \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY certs ./certs

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies and network tools
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    iproute2 \
    iputils-ping \
    net-tools \
    iptables \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/rover-rtc /app/rover-rtc
COPY --from=builder /app/certs /app/certs

# Make sure the binary is executable
RUN chmod +x /app/rover-rtc

ENTRYPOINT ["/app/rover-rtc"]
