# 📦 **Ramparts Proxy Installation Guide**

Multiple installation methods for different use cases and platforms.

## 🚀 **Quick Start (One-Line Install)**

```bash
# Automatic installer (detects your platform)
curl -sSL https://raw.githubusercontent.com/getjavelin/ramparts/main/scripts/install.sh | bash
```

## 📋 **Installation Methods**

### **Method 1: Cargo Install (Recommended for Developers)**

```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install ramparts
cargo install --git https://github.com/getjavelin/ramparts

# Verify installation
ramparts --version
```

### **Method 2: Pre-built Binaries (Recommended for Users)**

#### **Linux x64:**
```bash
curl -L https://github.com/getjavelin/ramparts/releases/latest/download/ramparts-v0.7.0-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv ramparts /usr/local/bin/
```

#### **macOS Intel:**
```bash
curl -L https://github.com/getjavelin/ramparts/releases/latest/download/ramparts-v0.7.0-x86_64-apple-darwin.tar.gz | tar -xz
sudo mv ramparts /usr/local/bin/
```

#### **macOS Apple Silicon:**
```bash
curl -L https://github.com/getjavelin/ramparts/releases/latest/download/ramparts-v0.7.0-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv ramparts /usr/local/bin/
```

#### **Windows:**
```powershell
# Download from GitHub releases
# Extract ramparts.exe to your PATH
```

### **Method 3: Docker (Recommended for Production)**

#### **Quick Start:**
```bash
# Set your API key
export JAVELIN_API_KEY="your-api-key"

# Run with Docker
docker run -d \
  --name ramparts-proxy \
  -p 8080:8080 \
  -e JAVELIN_API_KEY="$JAVELIN_API_KEY" \
  getjavelin/ramparts:latest \
  proxy 0.0.0.0:8080
```

#### **Docker Compose:**
```bash
# Download docker-compose.yml
curl -O https://raw.githubusercontent.com/getjavelin/ramparts/main/docker-compose.yml

# Set environment variables
export JAVELIN_API_KEY="your-api-key"

# Start services
docker-compose up -d
```

### **Method 4: Package Managers**

#### **Homebrew (macOS/Linux):**
```bash
# Coming soon
brew install getjavelin/tap/ramparts
```

#### **Chocolatey (Windows):**
```powershell
# Coming soon
choco install ramparts
```

#### **Snap (Linux):**
```bash
# Coming soon
sudo snap install ramparts
```

## ⚙️ **Configuration**

### **Required: API Key**
```bash
# Set your Javelin API key (required)
export JAVELIN_API_KEY="your-api-key-here"

# Alternative environment variable names (for compatibility)
export LLM_API_KEY="your-api-key-here"
export OPENAI_API_KEY="your-api-key-here"  # Legacy support
```

### **Optional: Advanced Configuration**
```bash
# Javelin API configuration
export JAVELIN_API_URL="https://api-dev.javelin.live"  # Default
export JAVELIN_TIMEOUT_SECONDS="30"                    # Default
export JAVELIN_FAIL_OPEN="true"                        # Default: fail open

# Proxy behavior
export PROXY_LOG_REQUESTS="true"                       # Default
export PROXY_CACHE_VALIDATIONS="false"                 # Default
export PROXY_MAX_REQUEST_SIZE="1048576"                # 1MB default

# Logging
export RUST_LOG="info"                                 # Default
```

## 🚀 **Usage**

### **Start the Proxy:**
```bash
# Basic usage
ramparts proxy 127.0.0.1:8080

# With custom configuration
JAVELIN_API_KEY="your-key" ramparts proxy 0.0.0.0:8080
```

### **Test Installation:**
```bash
# Health check
curl http://127.0.0.1:8080/health

# License status
curl http://127.0.0.1:8080/license

# MCP tools list
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
```

## 🔧 **Integration with MCP Clients**

### **Claude Desktop:**
```json
{
  "mcpServers": {
    "ramparts-proxy": {
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

### **Cursor/VS Code:**
```json
{
  "mcp": {
    "servers": {
      "ramparts-proxy": {
        "url": "http://localhost:8080/mcp"
      }
    }
  }
}
```

## 🐳 **Production Deployment**

### **Docker Swarm:**
```yaml
version: '3.8'
services:
  ramparts-proxy:
    image: getjavelin/ramparts:latest
    deploy:
      replicas: 3
      restart_policy:
        condition: on-failure
    ports:
      - "8080:8080"
    environment:
      - JAVELIN_API_KEY_FILE=/run/secrets/javelin_api_key
    secrets:
      - javelin_api_key

secrets:
  javelin_api_key:
    external: true
```

### **Kubernetes:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ramparts-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ramparts-proxy
  template:
    metadata:
      labels:
        app: ramparts-proxy
    spec:
      containers:
      - name: ramparts-proxy
        image: getjavelin/ramparts:latest
        ports:
        - containerPort: 8080
        env:
        - name: JAVELIN_API_KEY
          valueFrom:
            secretKeyRef:
              name: javelin-secret
              key: api-key
        command: ["ramparts", "proxy", "0.0.0.0:8080"]
```

## 🔍 **Troubleshooting**

### **Common Issues:**

1. **"API key required" error:**
   ```bash
   export JAVELIN_API_KEY="your-actual-api-key"
   ```

2. **"Connection refused" error:**
   ```bash
   # Check if proxy is running
   ps aux | grep ramparts
   
   # Check port availability
   netstat -tulpn | grep 8080
   ```

3. **"Permission denied" error:**
   ```bash
   # Use different port (>1024)
   ramparts proxy 127.0.0.1:8080
   ```

### **Debug Mode:**
```bash
RUST_LOG=debug ramparts proxy 127.0.0.1:8080
```

## 📞 **Support**

- **Documentation**: https://docs.getjavelin.com
- **Issues**: https://github.com/getjavelin/ramparts/issues
- **API Access**: https://www.getjavelin.com
- **Support**: support@getjavelin.com
