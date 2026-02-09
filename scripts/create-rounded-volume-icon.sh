#!/bin/bash
# Create a rounded macOS-style volume icon from the source icon

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOURCE_ICON="$PROJECT_ROOT/src-tauri/icons/icon-1024.png"
OUTPUT_ICON="$PROJECT_ROOT/src-tauri/icons/VolumeIcon.png"
FINAL_ICNS="$PROJECT_ROOT/src-tauri/icons/VolumeIcon.icns"

echo "Creating rounded volume icon from $SOURCE_ICON..."

# Create a 1024x1024 rounded rectangle mask with macOS-style corner radius
# macOS app icons use ~22.37% corner radius (229px at 1024px)
CORNER_RADIUS=229

# Create rounded icon with shadow
magick "$SOURCE_ICON" \
  \( +clone -alpha extract \
     -draw "fill black polygon 0,0 0,$CORNER_RADIUS $CORNER_RADIUS,0 \
            fill white circle $CORNER_RADIUS,$CORNER_RADIUS $CORNER_RADIUS,0" \
     \( +clone -flip \) -compose Multiply -composite \
     \( +clone -flop \) -compose Multiply -composite \
  \) -alpha off -compose CopyOpacity -composite \
  -background none \
  \( +clone -background black -shadow 60x8+0+16 \) +swap \
  -background none -layers merge +repage \
  "$OUTPUT_ICON"

echo "Created rounded PNG: $OUTPUT_ICON"

# Convert to .icns format using iconutil (macOS built-in)
# First create the iconset directory structure
ICONSET_DIR="$PROJECT_ROOT/src-tauri/icons/VolumeIcon.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Generate all required sizes for .icns
sips -z 16 16     "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_16x16.png" > /dev/null 2>&1
sips -z 32 32     "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_16x16@2x.png" > /dev/null 2>&1
sips -z 32 32     "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_32x32.png" > /dev/null 2>&1
sips -z 64 64     "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_32x32@2x.png" > /dev/null 2>&1
sips -z 128 128   "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_128x128.png" > /dev/null 2>&1
sips -z 256 256   "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_128x128@2x.png" > /dev/null 2>&1
sips -z 256 256   "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_256x256.png" > /dev/null 2>&1
sips -z 512 512   "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_256x256@2x.png" > /dev/null 2>&1
sips -z 512 512   "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_512x512.png" > /dev/null 2>&1
sips -z 1024 1024 "$OUTPUT_ICON" --out "$ICONSET_DIR/icon_512x512@2x.png" > /dev/null 2>&1

# Convert iconset to .icns
iconutil -c icns "$ICONSET_DIR" -o "$FINAL_ICNS"

# Clean up
rm -rf "$ICONSET_DIR"
rm "$OUTPUT_ICON"

echo "Created .icns file: $FINAL_ICNS"
echo "Done!"
