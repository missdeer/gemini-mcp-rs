#!/bin/bash

set -euo pipefail

BINARY_NAME="gemini-mcp-rs"
REPO="missdeer/$BINARY_NAME"
INSTALL_DIR="$HOME/.local/bin"

# Detect platform and architecture
detect_platform() {
    local OS=$(uname -s)
    local ARCH=$(uname -m)
    
    case "$OS" in
        Linux)
            PLATFORM="Linux"
            if [ "$ARCH" != "x86_64" ]; then
                echo "Error: Unsupported architecture: $ARCH (only x86_64 Linux is supported)"
                exit 1
            fi
            ARCH="x86_64"
            ;;
        Darwin)
            PLATFORM="Darwin"
            ARCH="universal"
            ;;
        *)
            echo "Error: Unsupported operating system: $OS"
            exit 1
            ;;
    esac
}

# Get latest version from GitHub Releases
get_latest_version() {
    local LATEST_URL="https://api.github.com/repos/${REPO}/releases/latest"
    
    if command -v jq &> /dev/null; then
        VERSION=$(curl -sSL "$LATEST_URL" | jq -r '.tag_name')
    else
        VERSION=$(curl -sSL "$LATEST_URL" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/' | head -1)
    fi
    
    if [ -z "$VERSION" ]; then
        echo "Error: Failed to fetch latest version"
        exit 1
    fi
    
    echo "$VERSION"
}

# Download and extract binary
download_and_extract() {
    local PLATFORM="$1"
    local ARCH="$2"
    local VERSION="$3"
    
    local FILENAME="${BINARY_NAME}_${PLATFORM}_${ARCH}.tar.gz"
    local DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"
    
    echo "Downloading ${VERSION}..."
    local TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT
    
    cd "$TEMP_DIR"
    curl -fsSL -o "$FILENAME" "$DOWNLOAD_URL"
    tar -xzf "$FILENAME"
    
    if [ ! -f "$BINARY_NAME" ]; then
        echo "Error: Binary not found in downloaded archive"
        exit 1
    fi
    
    # Install to user's local bin
    mkdir -p "$INSTALL_DIR"
    cp "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    echo "Installed to: $INSTALL_DIR/$BINARY_NAME"
    echo ""

    # Check if INSTALL_DIR is in PATH, if not suggest adding it
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "Please ensure '$INSTALL_DIR' is in your PATH."
        echo ""
        echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
    fi

    echo "$INSTALL_DIR/$BINARY_NAME"
}

# check the first argument is the path to the gemini-mcp-rs binary
if [ -n "${1-}" ]; then
    GEMINI_MCP_RS_PATH="$1"
elif [ -f "target/release/$BINARY_NAME" ]; then
    GEMINI_MCP_RS_PATH="target/release/$BINARY_NAME"
else
    # Binary not found locally, download from GitHub Releases
    echo "gemini-mcp-rs binary not found locally, downloading from GitHub Releases..."
    
    detect_platform
    VERSION=$(get_latest_version)
    GEMINI_MCP_RS_PATH=$(download_and_extract "$PLATFORM" "$ARCH" "$VERSION")
fi

# Add the gemini-mcp-rs server to the Claude Code MCP registry
CLAUDE_PATH=$(command -v claude 2>/dev/null || true)
if [ -n "$CLAUDE_PATH" ] && [ -x "$CLAUDE_PATH" ]; then
    "$CLAUDE_PATH" mcp add gemini-rs -s user --transport stdio -- "$GEMINI_MCP_RS_PATH"
else
    echo "Warning: claude CLI not found in PATH"
    echo "To register with Claude Code later, run:"
    echo "  claude mcp add gemini-rs -s user --transport stdio -- \"$GEMINI_MCP_RS_PATH\""
fi
