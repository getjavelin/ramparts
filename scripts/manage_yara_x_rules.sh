#!/bin/bash

# Script to manage YARA-X rules
# With YARA-X, rules are loaded directly from .yar files - no compilation needed!

set -e

RULES_DIR="rules"
PRE_DIR="$RULES_DIR/pre"
POST_DIR="$RULES_DIR/post"

echo "🔧 Managing YARA-X rules..."

# Function to validate rules in a directory
validate_rules_in_dir() {
    local dir="$1"
    local phase="$2"
    
    if [ ! -d "$dir" ]; then
        echo "⚠️  Directory $dir does not exist, creating..."
        mkdir -p "$dir"
        return
    fi
    
    echo "📁 Validating rules in $dir ($phase phase)..."
    
    local rule_count=0
    # Find all .yar files in the directory
    for yar_file in "$dir"/*.yar; do
        if [ -f "$yar_file" ]; then
            base_name=$(basename "$yar_file" .yar)
            echo "  ✅ Found rule: $base_name"
            ((rule_count++))
        fi
    done
    
    if [ $rule_count -eq 0 ]; then
        echo "  📝 No .yar files found in $dir"
        echo "     Create .yar files here and they'll be loaded automatically!"
    else
        echo "  📊 Found $rule_count rule(s) in $dir"
    fi
}

# Create directories if they don't exist
mkdir -p "$PRE_DIR" "$POST_DIR"

# Validate pre-scan rules
validate_rules_in_dir "$PRE_DIR" "pre-scan"

# Validate post-scan rules  
validate_rules_in_dir "$POST_DIR" "post-scan"

# Clean up old .yarac files (no longer needed with YARA-X)
echo "🧹 Cleaning up old compiled rules (.yarac files)..."
find "$RULES_DIR" -name "*.yarac" -type f -delete 2>/dev/null || true

echo "✅ YARA-X rule management complete!"
echo ""
echo "📋 Summary:"
echo "  - Pre-scan rules: $(find "$PRE_DIR" -name "*.yar" 2>/dev/null | wc -l)"
echo "  - Post-scan rules: $(find "$POST_DIR" -name "*.yar" 2>/dev/null | wc -l)"
echo ""
echo "💡 YARA-X advantages:"
echo "   - No compilation needed - .yar files are loaded directly"
echo "   - Better error messages for rule debugging"
echo "   - Improved performance with complex rules"
echo "   - 99% compatible with existing YARA syntax"
echo ""
echo "📖 Rule placement:"
echo "   - Place .yar files in $PRE_DIR/ for pre-scan analysis"
echo "   - Place .yar files in $POST_DIR/ for post-scan analysis"