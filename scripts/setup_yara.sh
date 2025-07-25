#!/bin/bash

# Script to set up YARA for local development and testing
# This script helps ensure YARA is properly installed and configured

set -e

echo "🔧 Setting up YARA for Ramparts development..."

# Detect OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "📱 Detected macOS"
    
    # Check if Homebrew is installed
    if ! command -v brew &> /dev/null; then
        echo "❌ Homebrew is not installed. Please install Homebrew first:"
        echo "   https://brew.sh/"
        exit 1
    fi
    
    # Install YARA via Homebrew
    echo "🍺 Installing YARA via Homebrew..."
    brew install yara
    
    # Set environment variables for development
    echo "🔧 Setting up environment variables..."
    
    # Check for Apple Silicon vs Intel
    if [[ $(uname -m) == "arm64" ]]; then
        # Apple Silicon
        export YARA_LIBRARY_PATH="/opt/homebrew/lib"
        export BINDGEN_EXTRA_CLANG_ARGS="-I/opt/homebrew/include"
        echo "✅ Set up for Apple Silicon (M1/M2)"
    else
        # Intel
        export YARA_LIBRARY_PATH="/usr/local/lib"
        export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/local/include"
        echo "✅ Set up for Intel Mac"
    fi
    
    echo "📝 Add these to your shell profile (.zshrc, .bash_profile, etc.):"
    echo "   export YARA_LIBRARY_PATH=\"$YARA_LIBRARY_PATH\""
    echo "   export BINDGEN_EXTRA_CLANG_ARGS=\"$BINDGEN_EXTRA_CLANG_ARGS\""
    
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "🐧 Detected Linux"
    
    # Check if we're on Ubuntu/Debian
    if command -v apt-get &> /dev/null; then
        echo "📦 Installing YARA via apt..."
        sudo apt-get update
        sudo apt-get install -y yara
    else
        echo "⚠️  Please install YARA manually for your Linux distribution"
        echo "   Visit: https://yara.readthedocs.io/en/stable/gettingstarted.html"
    fi
    
else
    echo "⚠️  Unsupported OS: $OSTYPE"
    echo "   Please install YARA manually"
    echo "   Visit: https://yara.readthedocs.io/en/stable/gettingstarted.html"
    exit 1
fi

# Test YARA installation
echo "🧪 Testing YARA installation..."
if command -v yara &> /dev/null; then
    echo "✅ YARA is installed and available"
    yara --version
else
    echo "❌ YARA is not available in PATH"
    exit 1
fi

# Test compilation
echo "🔨 Testing Rust compilation with YARA..."
if cargo check --features yara-scanning; then
    echo "✅ Rust compilation with YARA features successful"
else
    echo "❌ Rust compilation failed"
    echo "💡 Try setting the environment variables manually:"
    echo "   export YARA_LIBRARY_PATH=\"/opt/homebrew/lib\"  # or /usr/local/lib"
    echo "   export BINDGEN_EXTRA_CLANG_ARGS=\"-I/opt/homebrew/include\"  # or /usr/local/include"
    exit 1
fi

echo "🎉 YARA setup complete!"
echo ""
echo "💡 Next steps:"
echo "   1. Add the environment variables to your shell profile"
echo "   2. Run 'cargo build --release' to build the project"
echo "   3. Run 'cargo test' to run tests" 