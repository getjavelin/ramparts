#!/bin/bash

# Script to compile YARA source files (.yar) to compiled files (.yarac)
# This script should be run before using the dynamic YARA scanner

set -e

RULES_DIR="rules"
PRE_DIR="$RULES_DIR/pre"
POST_DIR="$RULES_DIR/post"

echo "🔧 Compiling YARA rules..."

# Function to compile rules in a directory
compile_rules_in_dir() {
    local dir="$1"
    local phase="$2"
    
    if [ ! -d "$dir" ]; then
        echo "⚠️  Directory $dir does not exist, skipping..."
        return
    fi
    
    echo "📁 Compiling rules in $dir ($phase phase)..."
    
    # Find all .yar files in the directory
    for yar_file in "$dir"/*.yar; do
        if [ -f "$yar_file" ]; then
            # Get the base name without extension
            base_name=$(basename "$yar_file" .yar)
            yarac_file="$dir/${base_name}.yarac"
            
            echo "  🔨 Compiling $yar_file -> $yarac_file"
            
            # Compile the .yar file to .yarac
            if yarac "$yar_file" "$yarac_file"; then
                echo "  ✅ Successfully compiled $base_name"
            else
                echo "  ❌ Failed to compile $base_name"
                exit 1
            fi
        fi
    done
}

# Compile pre-scan rules
compile_rules_in_dir "$PRE_DIR" "pre-scan"

# Compile post-scan rules
compile_rules_in_dir "$POST_DIR" "post-scan"

echo "✅ YARA rule compilation complete!"
echo ""
echo "📋 Summary:"
echo "  - Pre-scan rules: $(find "$PRE_DIR" -name "*.yarac" 2>/dev/null | wc -l)"
echo "  - Post-scan rules: $(find "$POST_DIR" -name "*.yarac" 2>/dev/null | wc -l)"
echo ""
echo "💡 Note: The dynamic YARA scanner will only load .yarac files."
echo "   Run this script whenever you add or modify .yar source files." 