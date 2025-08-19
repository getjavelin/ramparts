#!/bin/bash
set -e

# Ramparts Installation Script
# Supports multiple installation methods

REPO="getjavelin/ramparts"
BINARY_NAME="ramparts"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect OS and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case $os in
        linux*) OS="linux" ;;
        darwin*) OS="darwin" ;;
        msys*|mingw*|cygwin*) OS="windows" ;;
        *) log_error "Unsupported OS: $os"; exit 1 ;;
    esac
    
    case $arch in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) log_error "Unsupported architecture: $arch"; exit 1 ;;
    esac
    
    PLATFORM="${ARCH}-unknown-${OS}-gnu"
    if [[ "$OS" == "darwin" ]]; then
        PLATFORM="${ARCH}-apple-darwin"
    elif [[ "$OS" == "windows" ]]; then
        PLATFORM="${ARCH}-pc-windows-gnu"
        BINARY_NAME="ramparts.exe"
    fi
    
    log_info "Detected platform: $PLATFORM"
}

# Install via Cargo (recommended)
install_cargo() {
    log_info "Installing via Cargo..."
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    cargo install --git https://github.com/$REPO
    log_success "Installed via Cargo"
}

# Install pre-built binary
install_binary() {
    log_info "Installing pre-built binary..."
    
    # Get latest release
    local latest_release=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [[ -z "$latest_release" ]]; then
        log_error "Could not fetch latest release"
        exit 1
    fi
    
    log_info "Latest release: $latest_release"
    
    # Download binary
    local download_url="https://github.com/$REPO/releases/download/$latest_release/ramparts-$latest_release-$PLATFORM.tar.gz"
    if [[ "$OS" == "windows" ]]; then
        download_url="https://github.com/$REPO/releases/download/$latest_release/ramparts-$latest_release-$PLATFORM.zip"
    fi
    
    log_info "Downloading from: $download_url"
    
    local temp_dir=$(mktemp -d)
    cd "$temp_dir"
    
    if [[ "$OS" == "windows" ]]; then
        curl -L "$download_url" -o ramparts.zip
        unzip ramparts.zip
    else
        curl -L "$download_url" | tar -xz
    fi
    
    # Install to /usr/local/bin or user's bin directory
    local install_dir="/usr/local/bin"
    if [[ ! -w "$install_dir" ]]; then
        install_dir="$HOME/.local/bin"
        mkdir -p "$install_dir"
    fi
    
    mv "$BINARY_NAME" "$install_dir/"
    chmod +x "$install_dir/$BINARY_NAME"
    
    log_success "Installed to $install_dir/$BINARY_NAME"
    
    # Cleanup
    cd - > /dev/null
    rm -rf "$temp_dir"
}

# Install via Docker
install_docker() {
    log_info "Setting up Docker installation..."
    
    if ! command -v docker &> /dev/null; then
        log_error "Docker not found. Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    # Create docker-compose.yml
    cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  ramparts-proxy:
    image: getjavelin/ramparts:latest
    ports:
      - "8080:8080"
    environment:
      - JAVELIN_API_KEY=${JAVELIN_API_KEY}
      - JAVELIN_API_URL=${JAVELIN_API_URL:-https://api-dev.javelin.live}
    restart: unless-stopped
EOF
    
    log_success "Created docker-compose.yml"
    log_info "To start: docker-compose up -d"
    log_info "Set JAVELIN_API_KEY environment variable first!"
}

# Main installation logic
main() {
    echo "🚀 Ramparts MCP Proxy Installer"
    echo "================================"
    
    detect_platform
    
    # Check installation method preference
    if [[ "$1" == "cargo" ]]; then
        install_cargo
    elif [[ "$1" == "docker" ]]; then
        install_docker
    elif [[ "$1" == "binary" ]]; then
        install_binary
    else
        echo "Choose installation method:"
        echo "1) Cargo (recommended for developers)"
        echo "2) Pre-built binary (recommended for users)"
        echo "3) Docker (recommended for production)"
        
        read -p "Enter choice [1-3]: " choice
        case $choice in
            1) install_cargo ;;
            2) install_binary ;;
            3) install_docker ;;
            *) log_error "Invalid choice"; exit 1 ;;
        esac
    fi
    
    echo ""
    log_success "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "1. Set your Javelin API key:"
    echo "   export JAVELIN_API_KEY='your-api-key'"
    echo ""
    echo "2. Start the proxy:"
    echo "   ramparts proxy 127.0.0.1:8080"
    echo ""
    echo "3. Test the installation:"
    echo "   curl http://127.0.0.1:8080/health"
    echo ""
    echo "📚 Documentation: https://github.com/$REPO"
    echo "🔑 Get API key: https://www.getjavelin.com"
}

main "$@"
