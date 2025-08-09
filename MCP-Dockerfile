FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app

# Cache deps and build
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY rules ./rules
COPY assets ./assets
COPY build.rs ./

RUN cargo build --release

FROM ubuntu:24.04

# Install Docker CLI only (will use host Docker daemon via socket)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    && mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update \
    && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ramparts /app/
COPY --from=builder /app/rules /app/rules

# Default: run as MCP stdio server (MCP Toolkit/hosts connect over stdio)
ENTRYPOINT ["/app/ramparts", "mcp-stdio"]


