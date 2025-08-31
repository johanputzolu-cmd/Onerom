# Basic builds
cargo build
cargo build --release

# Builds of supported hardware variants
cargo build --release --no-default-features --features f401re
cargo build --release --no-default-features --features f405rg
cargo build --release --no-default-features --features f411re
cargo build --release --no-default-features --features f446re

# Builds of supported hardware variants including "control" support
cargo build --release --no-default-features --features f401re,control
cargo build --release --no-default-features --features f405rg,control
cargo build --release --no-default-features --features f411re,control
cargo build --release --no-default-features --features f446re,control

# Check with logging
DEFMT_LOG=trace cargo build
DEFMT_LOG=debug cargo build --release
DEFMT_LOG=info cargo build --release
DEFMT_LOG=warn cargo build --release
DEFMT_LOG=error cargo build --release

# Clippy
cargo clippy -- -D warnings
cargo clippy --no-default-features --features f401re -- -D warnings 

# Docs
cargo doc
