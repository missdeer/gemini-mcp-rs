#!/bin/bash

set -euo pipefail

BINARY_NAME="gemini-mcp-rs"
REPO="missdeer/$BINARY_NAME"
INSTALL_DIR="/opt/$BINARY_NAME"

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
    local VERSION=$(curl -sSL "$LATEST_URL" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/' | head -1)
    
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
    trap "rm -rf $TEMP_DIR" EXIT
    
    cd "$TEMP_DIR"
    curl -fsSL -o "$FILENAME" "$DOWNLOAD_URL"
    tar -xzf "$FILENAME"
    
    if [ ! -f "$BINARY_NAME" ]; then
        echo "Error: Binary not found in downloaded archive"
        exit 1
    fi
    
    # Install to /opt
    sudo mkdir -p "$INSTALL_DIR"
    sudo cp "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    
    echo "Installed to: $INSTALL_DIR/$BINARY_NAME"
    echo "$INSTALL_DIR/$BINARY_NAME"
}

# check the first argument is the path to the gemini-mcp-rs binary
if [ -n "$1" ]; then
    GEMINI_MCP_RS_PATH="$1"
fi

if [ -z "$GEMINI_MCP_RS_PATH" ]; then
    # Get the absolute path of the gemini-mcp-rs binary
    # if current os is Darwin, use $(pwd)/gemini-mcp-rs
    if [ "$(uname)" == "Darwin" ]; then
        GEMINI_MCP_RS_PATH=$(pwd)/gemini-mcp-rs
    fi
    if [ ! -f "$GEMINI_MCP_RS_PATH" ]; then
        GEMINI_MCP_RS_PATH=$(pwd)/target/release/gemini-mcp-rs
        if [ ! -f "$GEMINI_MCP_RS_PATH" ]; then
            # Binary not found locally, download from GitHub Releases
            echo "gemini-mcp-rs binary not found locally, downloading from GitHub Releases..."
            
            detect_platform
            VERSION=$(get_latest_version)
            GEMINI_MCP_RS_PATH=$(download_and_extract "$PLATFORM" "$ARCH" "$VERSION")
        fi
    fi
fi

# Add the gemini-mcp-rs server to the Claude Code MCP registry
CLAUDE_PATH=$(which claude)
if [ -f "$CLAUDE_PATH" ]; then
    "$CLAUDE_PATH" mcp add gemini-rs -s user --transport stdio -- "$GEMINI_MCP_RS_PATH"
else
    echo "Error: claude not found"
    exit 1
fi
