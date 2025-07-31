# Installation Guide

## Overview

Ramparts can be installed with or without YARA-X support depending on your security requirements. YARA-X provides advanced pattern-based security scanning and is recommended for comprehensive analysis.

## YARA-X Integration (Optional)

Ramparts uses YARA-X, a modern rewrite of YARA in Rust, for advanced pattern-based security scanning. YARA-X integration is **optional** but **recommended** for comprehensive security analysis.

### Key Benefits of YARA-X

- **Pure Rust**: No system dependencies required - everything is handled at compile time
- **Better Performance**: Optimized for complex security rules and mixed rule sets
- **Memory Safe**: Built with Rust's safety guarantees

## Installation Methods

### From crates.io (Recommended)

**With YARA-X support (recommended):**
```bash
cargo install ramparts
```

**Without YARA-X support (lighter installation):**
```bash
cargo install ramparts --no-default-features
```

### From Source

**Clone and install with YARA-X support:**
```bash
git clone https://github.com/getjavelin/ramparts.git
cd ramparts
cargo install --path .
```

**Clone and install without YARA-X support:**
```bash
git clone https://github.com/getjavelin/ramparts.git
cd ramparts
cargo install --path . --no-default-features
```

## Verification

Verify your installation:

```bash
ramparts --version
ramparts --help
```

## System Requirements

### Minimum Requirements
- Rust 1.70 or later
- 512MB RAM
- 100MB disk space

### Recommended Requirements
- Rust 1.75 or later
- 1GB RAM
- 500MB disk space (for YARA-X rules and cache)

## Platform Support

Ramparts supports all major platforms:

- **Linux** (x86_64, aarch64)
- **macOS** (Intel, Apple Silicon)
- **Windows** (x86_64)

## Configuration After Installation

After installation, you can initialize a configuration file:

```bash
ramparts init-config
```

This creates a `ramparts.yaml` file in your current directory with default settings.

## Upgrading

To upgrade to the latest version:

```bash
cargo install ramparts --force
```

## Uninstalling

To remove Ramparts:

```bash
cargo uninstall ramparts
```

## Troubleshooting Installation

### Common Issues

**Rust not installed:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Compilation errors:**
```bash
# Update Rust toolchain
rustup update

# Clean and retry
cargo clean
cargo install ramparts
```

**Permission errors on Linux/macOS:**
```bash
# Make sure cargo bin directory is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**YARA-X build failures:**
If you encounter YARA-X related build issues, try the no-default-features installation:
```bash
cargo install ramparts --no-default-features --force
```

### Getting Help

For installation issues:
1. Check our [troubleshooting guide](troubleshooting.md)
2. Open an issue on [GitHub](https://github.com/getjavelin/ramparts/issues)
3. Include your platform, Rust version, and error messages

## Next Steps

After installation:
1. Read the [usage guide](usage.md) for basic commands
2. Review [configuration options](configuration.md) for customization
3. Explore [security features](security-features.md) to understand what Ramparts detects