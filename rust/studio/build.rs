// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

fn main() {
    // Minimum macOS deployment target
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.13");

    // Re-run build script if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // Write built-time information
    built::write_built_file().expect("Failed to acquire build-time information");
}