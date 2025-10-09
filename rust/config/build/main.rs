// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use std::env;
use std::path::Path;

mod rom;

fn main() {
    // Get the manifest directory (where this crate's Cargo.toml is)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    // Get repo root (./../../)
    let repo_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to get repo root");

    rom::build(repo_root, manifest_path);
}
