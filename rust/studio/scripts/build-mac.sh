#!/bin/sh
set -e

# Builds the One ROM Studio application and dmg packages for macOS.
#
# Designed to be run from CI.
#
# DOES NOT SIGN OR NOTARIZE THE BUILDS.
#
# To run with code signing and notarization, use build-mac-arch.sh directly.
#
# Pre-requisites:
# - Rust:
#
# ```sh
#   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# ```
#
# - Homebrew:
#
# ```sh
#   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
# ```
#
# - Python 3 and pip: https://www.python.org/downloads/macos/

# Check we're running on macOS
if [ "$(uname -s)" != "Darwin" ]; then
    echo "Error: This script must be run on macOS" >&2
    exit 1
fi

#
# Setup
#

# Install the Rust targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Install cargo-bundle if not already installed
cargo install cargo-bundle --locked

# Install fileicon if not already installed
brew install fileicon

# Install python pip packages
python3 -m pip install --break-system-packages -r scripts/requirements.txt

#
# Clean previous builds
#

cargo clean --target x86_64-apple-darwin
cargo clean --target aarch64-apple-darwin
rm -fr dist/*.dmg

# Build for x86_64
scripts/build-mac-arch.sh x86_64 nosign

# Build for aarch64
scripts/build-mac-arch.sh aarch64 nosign
