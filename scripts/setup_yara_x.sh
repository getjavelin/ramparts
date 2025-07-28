#!/bin/bash

# Script to set up YARA-X for local development and testing
# YARA-X is a pure Rust implementation - no system dependencies needed!

set -e

echo "🔧 Setting up YARA-X for Ramparts development..."

echo "🦀 YARA-X is a pure Rust implementation - no system dependencies needed!"
echo "   This means no complex library installations or environment variables!"

# Test compilation
echo "🔨 Testing Rust compilation with YARA-X..."
if cargo check --features yara-x-scanning; then
    echo "✅ Rust compilation with YARA-X features successful"
else
    echo "❌ Rust compilation failed"
    echo "💡 Try updating your Rust toolchain:"
    echo "   rustup update stable"
    echo "   cargo update"
    exit 1
fi

echo "🎉 YARA-X setup complete!"
echo ""
echo "💡 Next steps:"
echo "   1. Run 'cargo build --release' to build the project"
echo "   2. Run 'cargo test' to run tests"
echo "   3. Place your .yar rules directly in rules/pre/ - no compilation needed!"
echo ""
echo "🔥 Benefits of YARA-X:"
echo "   - No system dependencies (pure Rust)"
echo "   - Better performance with complex rules" 
echo "   - 99% compatible with existing YARA rules"
echo "   - Memory safe and reliable"