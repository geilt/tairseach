#!/bin/bash
# Build, patch, and launch Tairseach.app
# Use this for testing permissions (dev mode won't register the app properly)

set -e

cd "$(dirname "$0")/.."

echo "🔨 Building Tairseach..."
npx tauri build 2>&1 | tail -20

APP_PATH="src-tauri/target/release/bundle/macos/Tairseach.app"

if [ -d "$APP_PATH" ]; then
    echo "✅ Build complete"
    
    # Patch Info.plist with privacy descriptions
    ./scripts/patch-info-plist.sh "$APP_PATH"
    
    echo "🚀 Launching Tairseach.app..."
    open "$APP_PATH"
else
    echo "❌ Build failed - app not found at $APP_PATH"
    exit 1
fi
