#!/bin/bash
# Post-build DMG polish script for Tairseach
# This script patches the DMG after Tauri builds it to:
# 1. Use a rounded volume icon
# 2. Hide the VolumeIcon.icns file from view

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"

echo "========================================"
echo "DMG Polish Script for Tairseach"
echo "========================================"

# Find the DMG file
DMG_FILE=$(find "$BUILD_DIR/dmg" -name "Tairseach_*.dmg" -type f | head -1)

if [[ -z "$DMG_FILE" ]]; then
    echo "Error: No DMG file found. Please run 'cargo tauri build' first."
    exit 1
fi

echo "Found DMG: $DMG_FILE"
DMG_NAME=$(basename "$DMG_FILE")
DMG_DIR=$(dirname "$DMG_FILE")

# Step 1: Convert DMG to read-write format
echo ""
echo "Step 1: Converting DMG to read-write format..."
RW_DMG="$DMG_DIR/Tairseach_rw_temp.dmg"
rm -f "$RW_DMG"
hdiutil convert "$DMG_FILE" -format UDRW -o "$RW_DMG"

# Step 2: Mount the read-write DMG
echo ""
echo "Step 2: Mounting read-write DMG..."
MOUNT_OUTPUT=$(hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" 2>&1)
DEV_NAME=$(echo "$MOUNT_OUTPUT" | grep -E '/dev/disk' | head -1 | awk '{print $1}')
MOUNT_DIR=$(echo "$MOUNT_OUTPUT" | grep -E '/Volumes/' | sed 's/.*\(\/Volumes\/.*\)$/\1/')

if [[ -z "$MOUNT_DIR" ]]; then
    echo "Error: Failed to mount DMG"
    rm -f "$RW_DMG"
    exit 1
fi

echo "Mounted at: $MOUNT_DIR"
echo "Device: $DEV_NAME"

# Step 3: Copy the rounded volume icon
echo ""
echo "Step 3: Installing rounded volume icon..."
if [[ -f "$PROJECT_ROOT/src-tauri/icons/VolumeIcon.icns" ]]; then
    cp "$PROJECT_ROOT/src-tauri/icons/VolumeIcon.icns" "$MOUNT_DIR/.VolumeIcon.icns"
    
    # Set file type
    SetFile -c icnC "$MOUNT_DIR/.VolumeIcon.icns"
    
    # CRITICAL: Hide the VolumeIcon.icns file
    SetFile -a V "$MOUNT_DIR/.VolumeIcon.icns"
    
    # Mark the volume as having a custom icon
    SetFile -a C "$MOUNT_DIR"
    
    echo "✓ Volume icon installed and hidden"
else
    echo "Warning: VolumeIcon.icns not found. Skipping volume icon installation."
    echo "Run ./scripts/create-rounded-volume-icon.sh first."
fi

# Step 4: Verify layout
echo ""
echo "Step 4: Verifying DMG contents..."
ls -la "$MOUNT_DIR" | grep -v "^d" || true

# Step 5: Unmount
echo ""
echo "Step 5: Unmounting DMG..."
hdiutil detach "$DEV_NAME"

# Step 6: Compress the modified DMG
echo ""
echo "Step 6: Compressing final DMG..."
rm -f "$DMG_FILE"

hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_FILE"

# Clean up temp file
rm -f "$RW_DMG"

echo ""
echo "========================================"
echo "✓ DMG polish complete!"
echo "Location: $DMG_FILE"
echo ""
echo "Changes applied:"
echo "  • Rounded volume icon installed"
echo "  • VolumeIcon.icns hidden from view"
echo "  • DMG layout preserved"
echo "========================================"
