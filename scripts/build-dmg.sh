#!/bin/bash
# Build Tairseach DMG with all polish fixes applied

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "========================================"
echo "Building Tairseach DMG with polish fixes"
echo "========================================"

# Step 1: Ensure rounded volume icon exists
echo ""
echo "Step 1: Generating rounded volume icon..."
if [[ ! -f "$PROJECT_ROOT/src-tauri/icons/VolumeIcon.icns" ]]; then
    "$SCRIPT_DIR/create-rounded-volume-icon.sh"
else
    echo "VolumeIcon.icns already exists, skipping generation."
fi

# Step 2: Build the app
echo ""
echo "Step 2: Building Tairseach with cargo tauri build..."
cd "$PROJECT_ROOT"
cargo tauri build

# Step 3: Find and patch the bundle_dmg.sh script
echo ""
echo "Step 3: Patching bundle_dmg.sh to hide VolumeIcon.icns..."
BUNDLE_SCRIPT="$PROJECT_ROOT/src-tauri/target/release/bundle/dmg/bundle_dmg.sh"

if [[ -f "$BUNDLE_SCRIPT" ]]; then
    "$SCRIPT_DIR/patch-dmg-script.sh" "$BUNDLE_SCRIPT"
    
    # Step 4: Re-run the patched DMG build script
    echo ""
    echo "Step 4: Rebuilding DMG with patched script..."
    cd "$PROJECT_ROOT/src-tauri/target/release/bundle/dmg"
    
    # Remove the old DMG
    rm -f Tairseach_*.dmg
    
    # Re-run the bundle script
    ./bundle_dmg.sh "Tairseach_0.1.0_aarch64.dmg" "$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Tairseach.app"
    
    echo ""
    echo "========================================"
    echo "✓ DMG build complete!"
    echo "Location: $PROJECT_ROOT/src-tauri/target/release/bundle/dmg/Tairseach_0.1.0_aarch64.dmg"
    echo "========================================"
else
    echo "Warning: bundle_dmg.sh not found. The build may have failed or used a different path."
    echo "Searching for DMG files..."
    find "$PROJECT_ROOT/src-tauri/target" -name "*.dmg" -type f
fi
