#!/bin/sh
set -e

# Builds the One ROM Studio application and dmg packages for Linux.
#
# Pre-requisites:
# - Rust:
#
# ```sh
#   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# ```

# Check we're running on Linux
if [ "$(uname -s)" != "Linux" ]; then
    echo "Error: This script must be run on Linux" >&2
    exit 1
fi

#
# Setup
#

# Install required packages
sudo apt update && sudo apt install -y libudev-dev libusb-1.0-0-dev gcc-aarch64-linux-gnu

# Also need aarch64 libudev-dev and libusb files
sudo dpkg --add-architecture arm64
sudo apt update && sudo apt install -y libudev-dev:arm64 libusb-1.0-0-dev:arm64

# Install the Rust targets
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu

# Install cargo-packager if not already installed
cargo install cargo-packager --locked

#
# Clean previous builds
#

cargo clean --target x86_64-unknown-linux-gnu
cargo clean --target aarch64-unknown-linux-gnu
rm -fr dist/*.deb

#
# Intel silicon (x86_64)
#

# Build One ROM Studio
PACKAGER_TARGET="x86_64-unknown-linux-gnu"
echo "Building for target: $PACKAGER_TARGET"
cargo build --release --target $PACKAGER_TARGET

# Package as a dmg
echo "Packaging dmg for target: $PACKAGER_TARGET"
cargo packager --release --target $PACKAGER_TARGET --formats deb

echo "Linux x86_64 build complete."

#
# ARM silicon (aarch64)
#

# Build One ROM Studio
# Note: Requires setting PKG_CONFIG_SYSROOT_DIR and PKG_CONFIG_PATH to find
# the arm64 libudev-dev files
PACKAGER_TARGET="aarch64-unknown-linux-gnu"
export PKG_CONFIG_SYSROOT_DIR=/
export PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
echo "Building for target: $PACKAGER_TARGET"
cargo build --release --target $PACKAGER_TARGET

# Package as a dmg
echo "Packaging dmg for target: $PACKAGER_TARGET"
cargo packager --release --target $PACKAGER_TARGET --formats deb
echo "Linux ARM64 build complete."

#
# Inject deb scripts
#
echo "Injecting deb scripts into generated .deb files..."
scripts/inject-deb-scripts.sh dist

echo "Build complete."