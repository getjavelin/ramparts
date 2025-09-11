# Multi-stage build optimized for layer caching
# Stage 1: Create base runtime with all language runtimes (cached layer)
FROM ubuntu:24.04 as runtime-base

# Install system dependencies and Docker CLI (cached layer)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    wget \
    git \
    build-essential \
    && mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update \
    && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Install Python 3.12 and pip (cached layer)
RUN apt-get update && apt-get install -y \
    python3.12 \
    python3.12-venv \
    python3-pip \
    && ln -sf /usr/bin/python3.12 /usr/bin/python \
    && ln -sf /usr/bin/python3.12 /usr/bin/python3 \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20 LTS and npm (cached layer)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest \
    && rm -rf /var/lib/apt/lists/*

# Install TypeScript and common Node.js tools globally (cached layer)
RUN npm install -g \
    typescript \
    ts-node \
    @types/node \
    && npm cache clean --force

# Install Go 1.21 (cached layer)
RUN wget https://go.dev/dl/go1.21.6.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.21.6.linux-amd64.tar.gz \
    && rm go1.21.6.linux-amd64.tar.gz
ENV PATH="/usr/local/go/bin:${PATH}"

# Install Java 21 LTS (OpenJDK) (cached layer)
RUN apt-get update && apt-get install -y \
    openjdk-21-jdk \
    maven \
    gradle \
    && rm -rf /var/lib/apt/lists/*
# Set JAVA_HOME dynamically based on architecture
RUN JAVA_HOME_PATH=$(find /usr/lib/jvm -name "java-21-openjdk-*" -type d | head -1) && \
    echo "export JAVA_HOME=${JAVA_HOME_PATH}" >> /etc/environment && \
    echo "export PATH=\${JAVA_HOME}/bin:\${PATH}" >> /etc/environment
ENV JAVA_HOME="/usr/lib/jvm/java-21-openjdk-arm64"
ENV PATH="${JAVA_HOME}/bin:${PATH}"

# Install .NET 8 LTS (cached layer)
RUN wget https://packages.microsoft.com/config/ubuntu/24.04/packages-microsoft-prod.deb -O packages-microsoft-prod.deb \
    && dpkg -i packages-microsoft-prod.deb \
    && rm packages-microsoft-prod.deb \
    && apt-get update \
    && apt-get install -y dotnet-sdk-8.0 \
    && rm -rf /var/lib/apt/lists/*

# Install PHP 8.3 with Composer (cached layer)
RUN apt-get update && apt-get install -y \
    php8.3 \
    php8.3-cli \
    php8.3-common \
    php8.3-curl \
    php8.3-mbstring \
    php8.3-xml \
    unzip \
    && curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer \
    && rm -rf /var/lib/apt/lists/*

# Install Ruby 3.2 with Bundler (cached layer)
RUN apt-get update && apt-get install -y \
    ruby3.2 \
    ruby3.2-dev \
    rubygems \
    && gem install bundler \
    && rm -rf /var/lib/apt/lists/*

# Create symlinks for common command names (cached layer)
RUN ln -sf /usr/bin/ruby3.2 /usr/bin/ruby \
    && ln -sf /usr/bin/php8.3 /usr/bin/php

# Stage 2: Rust builder (separate for dependency caching)
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/ramparts*

# Copy actual source code (this layer changes most often)
COPY src ./src
COPY rules ./rules
COPY assets ./assets
COPY build.rs ./

# Build the actual application
RUN cargo build --release

# Stage 3: Final runtime stage
FROM runtime-base

WORKDIR /app
COPY --from=builder /app/target/release/ramparts /app/
COPY --from=builder /app/rules /app/rules

# Default: run as MCP stdio server (MCP Toolkit/hosts connect over stdio)
ENTRYPOINT ["/app/ramparts", "mcp-stdio"]


