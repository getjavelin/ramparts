#!/bin/bash
set -e

# Ramparts Release Builder
# Builds binaries for multiple platforms

VERSION=${1:-$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')}
echo "Building Ramparts v$VERSION for multiple platforms..."

# Create release directory
mkdir -p releases

# Build for different targets
TARGETS=(
    "x86_64-unknown-linux-gnu"      # Linux x64
    "aarch64-unknown-linux-gnu"     # Linux ARM64
    "x86_64-apple-darwin"           # macOS Intel
    "aarch64-apple-darwin"          # macOS Apple Silicon
    "x86_64-pc-windows-gnu"         # Windows x64
)

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    
    # Install target if not present
    rustup target add $target 2>/dev/null || true
    
    # Build release binary
    cargo build --release --target $target
    
    # Create platform-specific archive
    case $target in
        *windows*)
            BINARY_NAME="ramparts.exe"
            ARCHIVE_NAME="ramparts-v$VERSION-$target.zip"
            cp "target/$target/release/$BINARY_NAME" "releases/"
            cd releases
            zip "$ARCHIVE_NAME" "$BINARY_NAME"
            rm "$BINARY_NAME"
            cd ..
            ;;
        *)
            BINARY_NAME="ramparts"
            ARCHIVE_NAME="ramparts-v$VERSION-$target.tar.gz"
            cp "target/$target/release/$BINARY_NAME" "releases/"
            cd releases
            tar -czf "$ARCHIVE_NAME" "$BINARY_NAME"
            rm "$BINARY_NAME"
            cd ..
            ;;
    esac
    
    echo "Created: releases/$ARCHIVE_NAME"
done

# Create checksums
cd releases
sha256sum * > checksums.txt
echo "Created checksums.txt"

echo ""
echo "Release build complete! Files in releases/:"
ls -la

echo ""
echo "Installation instructions:"
echo "1. Download the appropriate binary for your platform"
echo "2. Extract the archive"
echo "3. Move the binary to your PATH"
echo "4. Set JAVELIN_API_KEY environment variable"
echo "5. Run: ramparts proxy 127.0.0.1:8080"
