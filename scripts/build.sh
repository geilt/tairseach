#!/bin/bash
# Tairseach Build Script
# Performs a complete build, sign, and reset cycle
#
# Usage: ./scripts/build.sh [--dev]
#   --dev    Run in development mode after build (optional)

set -e

# Configuration
APP_NAME="Tairseach"
BUNDLE_ID="com.naonur.tairseach"
SIGNING_IDENTITY="Developer ID Application: ALEXANDER DAVID CONROY (ANRLR4YMQV)"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENTITLEMENTS="$PROJECT_ROOT/src-tauri/entitlements.plist"
APP_BUNDLE="$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Tairseach.app"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Tairseach Build Script                         ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

cd "$PROJECT_ROOT"

# Step 1: Kill any running instances
echo -e "${YELLOW}[1/6]${NC} Stopping any running Tairseach instances..."
pkill -f "Tairseach" 2>/dev/null || true
pkill -f "tairseach" 2>/dev/null || true
sleep 1
echo -e "${GREEN}      ✓ Done${NC}"

# Step 2: Build frontend and Rust backend
echo -e "${YELLOW}[2/6]${NC} Building frontend and Rust backend..."
echo "      This may take 1-2 minutes..."
npm run tauri build 2>&1 | while IFS= read -r line; do
    # Show progress indicators
    if [[ "$line" == *"Compiling"* ]]; then
        echo -ne "\r      Compiling: $(echo "$line" | grep -oP '(?<=Compiling )[^ ]+' | head -1)                    "
    elif [[ "$line" == *"Finished"* ]]; then
        echo -e "\r      ${GREEN}✓ Rust build complete${NC}                              "
    elif [[ "$line" == *"Bundling"*".app"* ]]; then
        echo -e "      ${GREEN}✓ App bundle created${NC}"
    elif [[ "$line" == *"error"* ]]; then
        echo -e "\r      ${RED}✗ Error: $line${NC}"
    fi
done

# Check if build succeeded
if [ ! -d "$APP_BUNDLE" ]; then
    echo -e "${RED}[ERROR] Build failed - app bundle not found${NC}"
    exit 1
fi
echo -e "${GREEN}      ✓ Build complete${NC}"

# Step 3: Code sign with Developer ID
echo -e "${YELLOW}[3/6]${NC} Signing with Developer ID certificate..."
codesign --force --deep --options runtime \
    --entitlements "$ENTITLEMENTS" \
    -s "$SIGNING_IDENTITY" \
    "$APP_BUNDLE" 2>&1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}      ✓ Code signing complete${NC}"
else
    echo -e "${RED}[ERROR] Code signing failed${NC}"
    exit 1
fi

# Step 4: Verify signature
echo -e "${YELLOW}[4/6]${NC} Verifying code signature..."
VERIFY_OUTPUT=$(codesign -dv "$APP_BUNDLE" 2>&1)
if echo "$VERIFY_OUTPUT" | grep -q "Signature="; then
    TEAM_ID=$(echo "$VERIFY_OUTPUT" | grep "TeamIdentifier" | cut -d= -f2)
    echo -e "${GREEN}      ✓ Signature verified (Team: $TEAM_ID)${NC}"
else
    echo -e "${RED}[ERROR] Signature verification failed${NC}"
    exit 1
fi

# Step 5: Reset TCC permissions
echo -e "${YELLOW}[5/6]${NC} Resetting TCC permissions cache..."
tccutil reset All "$BUNDLE_ID" 2>/dev/null || true
echo -e "${GREEN}      ✓ TCC permissions reset${NC}"

# Step 6: Summary
echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    BUILD SUCCESSFUL                      ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "App location: ${BLUE}$APP_BUNDLE${NC}"
echo ""
echo "To run the app:"
echo -e "  ${YELLOW}open \"$APP_BUNDLE\"${NC}"
echo ""

# Optional: Run in dev mode
if [[ "$1" == "--dev" ]]; then
    echo -e "${YELLOW}Starting app in development mode...${NC}"
    open "$APP_BUNDLE"
fi
