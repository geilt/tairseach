#!/bin/bash
# Tairseach Deploy to Bolcain
# Build on Croí and deploy to Bolcain (MacBook Pro M4 Max) for testing
#
# Usage: ./scripts/deploy-bolcain.sh [OPTIONS]
#
# Options:
#   --skip-build    Deploy existing bundle without rebuilding
#   --launch        Open the app on Bolcain after deployment
#   --help          Show this help message
#
# Requirements:
#   - SSH access to bolcain.local (or bolcain)
#   - Bolcain available as OpenClaw paired node

set -e

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Tairseach.app"
MCP_BINARY="$PROJECT_ROOT/src-tauri/target/release/tairseach-mcp"
MANIFESTS_DIR="$PROJECT_ROOT/manifests"
REMOTE_HOST="bolcain.local"
REMOTE_USER="${REMOTE_USER:-geilt}"
REMOTE_APP="Applications/Tairseach.app"
REMOTE_MANIFESTS="~/.tairseach/manifests/"
REMOTE_MCP_BIN="~/.tairseach/bin/tairseach-mcp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Parse flags
SKIP_BUILD=false
LAUNCH=false

show_help() {
    echo -e "${BLUE}Tairseach Deploy to Bolcain${NC}"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --skip-build    Deploy existing bundle without rebuilding"
    echo "  --launch        Open the app on Bolcain after deployment"
    echo "  --help          Show this help message"
    echo ""
    echo "This script builds Tairseach on Croí and deploys it to Bolcain for testing."
    exit 0
}

for arg in "$@"; do
    case $arg in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --launch)
            LAUNCH=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║         Tairseach Deploy to Bolcain                     ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Build (or skip)
if [ "$SKIP_BUILD" = false ]; then
    echo -e "${YELLOW}[1/5]${NC} Building Tairseach..."
    
    BUILD_SCRIPT="$PROJECT_ROOT/scripts/build.sh"
    if [ ! -f "$BUILD_SCRIPT" ]; then
        echo -e "${RED}[ERROR] Build script not found: $BUILD_SCRIPT${NC}"
        exit 1
    fi
    
    # Run build script
    "$BUILD_SCRIPT"
    
    if [ ! -d "$APP_BUNDLE" ]; then
        echo -e "${RED}[ERROR] Build failed - app bundle not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}      ✓ Build complete${NC}"
else
    echo -e "${YELLOW}[1/5]${NC} Skipping build (--skip-build)"
    
    # Verify bundle exists
    if [ ! -d "$APP_BUNDLE" ]; then
        echo -e "${RED}[ERROR] App bundle not found: $APP_BUNDLE${NC}"
        echo -e "${RED}       Run without --skip-build to build first${NC}"
        exit 1
    fi
    echo -e "${GREEN}      ✓ Using existing bundle${NC}"
fi

# Step 2: Verify SSH connectivity
echo -e "${YELLOW}[2/5]${NC} Verifying connection to Bolcain..."
if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "$REMOTE_USER@$REMOTE_HOST" "echo 'Connected'" &>/dev/null; then
    # Try alternate hostname
    REMOTE_HOST="bolcain"
    if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "$REMOTE_USER@$REMOTE_HOST" "echo 'Connected'" &>/dev/null; then
        echo -e "${RED}[ERROR] Cannot connect to Bolcain${NC}"
        echo -e "${RED}       Tried: bolcain.local and bolcain${NC}"
        echo -e "${RED}       Ensure SSH keys are configured and Bolcain is online${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}      ✓ Connected to $REMOTE_HOST${NC}"

# Step 3: Deploy app bundle
echo -e "${YELLOW}[3/5]${NC} Deploying app bundle to Bolcain..."
echo "      Target: $REMOTE_HOST:$REMOTE_APP"

# Create parent directory and sync
ssh "$REMOTE_USER@$REMOTE_HOST" "mkdir -p ~/Applications" 2>/dev/null || true

rsync -avz --delete \
    --info=progress2 \
    "$APP_BUNDLE/" \
    "$REMOTE_USER@$REMOTE_HOST:$REMOTE_APP/" 2>&1 | \
    grep -v "^sending incremental file list$" | \
    while IFS= read -r line; do
        if [[ "$line" == *"total size"* ]]; then
            echo -e "      ${GREEN}✓ App bundle deployed${NC}"
        elif [[ "$line" =~ ^[0-9]+% ]]; then
            echo -ne "\r      Progress: $line        "
        fi
    done

# Ensure final newline
echo ""

# Step 4: Deploy manifests
echo -e "${YELLOW}[4/5]${NC} Deploying manifests to Bolcain..."
if [ -d "$MANIFESTS_DIR" ]; then
    ssh "$REMOTE_USER@$REMOTE_HOST" "mkdir -p ~/.tairseach/manifests" 2>/dev/null || true
    
    rsync -avz --delete \
        "$MANIFESTS_DIR/" \
        "$REMOTE_USER@$REMOTE_HOST:$REMOTE_MANIFESTS" >/dev/null 2>&1
    
    echo -e "${GREEN}      ✓ Manifests deployed${NC}"
else
    echo -e "${YELLOW}      ⚠ Manifests directory not found (skipping)${NC}"
fi

# Step 5: Deploy MCP binary (if exists)
if [ -f "$MCP_BINARY" ]; then
    echo -e "${YELLOW}[5/5]${NC} Deploying MCP bridge binary..."
    
    ssh "$REMOTE_USER@$REMOTE_HOST" "mkdir -p ~/.tairseach/bin" 2>/dev/null || true
    
    rsync -avz \
        "$MCP_BINARY" \
        "$REMOTE_USER@$REMOTE_HOST:$REMOTE_MCP_BIN" >/dev/null 2>&1
    
    # Make executable
    ssh "$REMOTE_USER@$REMOTE_HOST" "chmod +x $REMOTE_MCP_BIN" 2>/dev/null || true
    
    echo -e "${GREEN}      ✓ MCP binary deployed${NC}"
else
    echo -e "${YELLOW}[5/5]${NC} MCP binary not found (skipping)"
fi

# Summary
echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              DEPLOYMENT SUCCESSFUL                       ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Deployed to: ${BLUE}$REMOTE_USER@$REMOTE_HOST${NC}"
echo -e "App location: ${BLUE}$REMOTE_APP${NC}"
echo -e "Manifests: ${BLUE}~/.tairseach/manifests/${NC}"
if [ -f "$MCP_BINARY" ]; then
    echo -e "MCP binary: ${BLUE}$REMOTE_MCP_BIN${NC}"
fi
echo ""

# Optional: Launch on Bolcain
if [ "$LAUNCH" = true ]; then
    echo -e "${YELLOW}Launching Tairseach on Bolcain...${NC}"
    ssh "$REMOTE_USER@$REMOTE_HOST" "open ~/'$REMOTE_APP'" 2>/dev/null
    echo -e "${GREEN}✓ App launched${NC}"
    echo ""
fi

echo "To run manually on Bolcain:"
echo -e "  ${CYAN}ssh $REMOTE_USER@$REMOTE_HOST \"open '$REMOTE_APP'\"${NC}"
echo ""
