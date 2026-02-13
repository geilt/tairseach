#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Building 1Password helper binary..."

# Initialize go.mod if it doesn't exist
if [ ! -f go.mod ]; then
    echo "Initializing Go module..."
    go mod init tairseach-op-helper
    go get github.com/1password/onepassword-sdk-go@latest
fi

# Build for darwin/arm64
echo "Compiling for darwin/arm64..."
GOOS=darwin GOARCH=arm64 go build -o ../../bin/op-helper main.go

echo "✅ Binary built: src-tauri/bin/op-helper"
ls -lh ../../bin/op-helper
