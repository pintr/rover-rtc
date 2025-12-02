# Lightweight Dockerfile using local build
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

# Copy pre-built binary from local target directory
COPY target/release/rover-rtc /app/rover-rtc
COPY certs /app/certs

# Make sure the binary is executable
RUN chmod +x /app/rover-rtc

ENTRYPOINT ["/app/rover-rtc"]
