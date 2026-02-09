#!/bin/bash
# Patch the Tauri-generated bundle_dmg.sh to hide VolumeIcon.icns

set -e

BUNDLE_SCRIPT="$1"

if [[ ! -f "$BUNDLE_SCRIPT" ]]; then
    echo "Error: bundle_dmg.sh not found at $BUNDLE_SCRIPT"
    exit 1
fi

echo "Patching $BUNDLE_SCRIPT to hide VolumeIcon.icns..."

# Check if already patched
if grep -q "SetFile -a V.*VolumeIcon.icns" "$BUNDLE_SCRIPT"; then
    echo "Already patched, skipping."
    exit 0
fi

# Add the SetFile -a V command after the VolumeIcon.icns is copied
# Find the line with 'SetFile -c icnC "$MOUNT_DIR/.VolumeIcon.icns"'
# and add 'SetFile -a V "$MOUNT_DIR/.VolumeIcon.icns"' after it

perl -i -pe 's/(SetFile -c icnC "\$MOUNT_DIR\/\.VolumeIcon\.icns")/$1\n\tSetFile -a V "\$MOUNT_DIR\/.VolumeIcon.icns"/' "$BUNDLE_SCRIPT"

echo "Patch applied successfully!"
