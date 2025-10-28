#!/bin/sh
set -e

# Builds One ROM Studio for macOS for a specified architecture
#
# Required env variables:
# - CODESIGN_IDENTITY: The codesign identity to use for signing the app.
#   This is the "name" of the certificate including "Developer ID ..."
#
# The notarytool uses a keychain profile named "onerom-notarization" for
# notarization. This profile must be set up in advance with the appropriate
# Apple ID and app-specific password using a command line:
#
# ```sh
# xcrun notarytool store-credentials "onerom-notarization" \
#  --apple-id your@email.com \
#  --team-id XXXXXXXXXX \
#  --password xxxx-xxxx-xxxx-xxxx
# ```
#
# - Team ID matches that on the Developer ID Application certificate.
# - The app-specific password is generated in the Apple ID account settings.

# Variables
DIST_DIR="dist"

# Check we're running on macOS
if [ "$(uname -s)" != "Darwin" ]; then
    echo "Error: This script must be run on macOS" >&2
    exit 1
fi

# Check arg is set to architecture
if [ -z "$1" ]; then
    echo "Usage: $0 <architecture>" >&2
    echo "Where <architecture> is one of: x86_64, aarch64" >&2
    exit 1
fi
ARCH="$1"
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "aarch64" ]; then
    echo "Error: Invalid architecture specified: $ARCH" >&2
    echo "Valid architectures are: x86_64, aarch64" >&2
    exit 1
fi

# Set packager target from architecture
if [ "$ARCH" = "x86_64" ]; then
    PACKAGER_TARGET="x86_64-apple-darwin"
    DMG_ARCH="x64"
else
    PACKAGER_TARGET="aarch64-apple-darwin"
    DMG_ARCH="arm64"
fi

# Set signing mode
SIGN="$2"
if [ "$SIGN" = "nosign" ]; then
    echo "!!! WARNING: Building without code signing or notarization" >&2
    CODESIGN_IDENTITY=""
else
    # Check codesign identity is set
    if [ -z "${CODESIGN_IDENTITY:-}" ]; then
        echo "Error: CODESIGN_IDENTITY environment variable must be set"
        exit 1
    fi
fi

# Get version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Set DMG filename
DMG_FILE="One ROM Studio_${VERSION}_${ARCH}.dmg"
DMG_PATH="${DIST_DIR}/${DMG_FILE}"

echo "Building One ROM Studio version: $VERSION"
echo "Building for architecture: $ARCH/$DMG_ARCH"
echo "Building dmg: $DMG_PATH"

# Unlock security keychain if signing is enabled
if [ -n "$CODESIGN_IDENTITY" ]; then
    echo "Unlocking keychain for code signing..."
    security unlock-keychain
fi

# Delete old DMG
if [ -f "${DMG_PATH}" ]; then
    echo "Removing old dmg: ${DMG_PATH}"
    rm -f "${DMG_PATH}"
fi

# Build One ROM Studio
echo "Building One ROM Studio for target: $PACKAGER_TARGET"
cargo build --release --target $PACKAGER_TARGET

# Package as a .app bundle
echo "Bundling dmg for target: $PACKAGER_TARGET"
cargo bundle --release --target $PACKAGER_TARGET
APP_FILE="../target/$PACKAGER_TARGET/release/bundle/osx/One ROM Studio.app"
echo "Built app file: $APP_FILE"

# Check if signing is enabled
if [ -n "$CODESIGN_IDENTITY" ]; then
    # Sign the app
    echo "Signing app..."
    codesign --deep --verify --options runtime --timestamp --sign "$CODESIGN_IDENTITY" ../target/$PACKAGER_TARGET/release/bundle/osx/One\ ROM\ Studio.app
fi

# Create the dmg
echo "Creating dmg..."
scripts/create-dmg.py \
    --app-bundle "$APP_FILE" \
    --output "${DMG_FILE}" \
    --dist-dir ${DIST_DIR} \

# Check if signing is enabled
if [ -n "$CODESIGN_IDENTITY" ]; then
    # Notarize the dmg
    echo "Notarizing dmg..."
    xcrun notarytool submit "$DMG_PATH" \
        --keychain-profile "onerom-notarization" \
        --wait

    # Staple the notarization ticket to the dmg
    echo "Stapling dmg..."
    xcrun stapler staple "$DMG_PATH"

    # Finished
    echo "Built and notarized dmg: $DMG_PATH"
else
    echo "Built dmg without notarization: $DMG_PATH"
fi