#!/bin/bash
# Check that versions are in sync across files

set -e

CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
NPM_VERSION=$(node -p "require('./npm/gemini-mcp-rs/package.json').version")
SERVER_VERSION=$(node -p "require('./server.json').version")

echo "Cargo.toml version: $CARGO_VERSION"
echo "npm/gemini-mcp-rs/package.json version: $NPM_VERSION"
echo "server.json version: $SERVER_VERSION"

# Check platform package versions
PLATFORMS=("darwin-universal" "linux-x64" "linux-arm64" "win32-x64" "win32-arm64")
for platform in "${PLATFORMS[@]}"; do
    PLATFORM_VERSION=$(node -p "require('./npm/platforms/$platform/package.json').version")
    echo "npm/platforms/$platform/package.json version: $PLATFORM_VERSION"
    if [ "$CARGO_VERSION" != "$PLATFORM_VERSION" ]; then
        echo "Error: Version mismatch in platform package $platform!"
        exit 1
    fi
done

# Check optionalDependencies versions match
for platform in "${PLATFORMS[@]}"; do
    DEP_VERSION=$(node -p "require('./npm/gemini-mcp-rs/package.json').optionalDependencies['@gemini-mcp-rs/$platform']")
    if [ "$CARGO_VERSION" != "$DEP_VERSION" ]; then
        echo "Error: optionalDependencies version mismatch for @gemini-mcp-rs/$platform!"
        exit 1
    fi
done

if [ "$CARGO_VERSION" != "$NPM_VERSION" ] || [ "$CARGO_VERSION" != "$SERVER_VERSION" ]; then
    echo "Error: Version mismatch!"
    exit 1
fi

echo "✓ All versions match: $CARGO_VERSION"

