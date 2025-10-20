// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
}