# Changelog

## v0.1.1 - 2025-10-30

- Built with rustc 1.90
- Mac and Windows releases now signed.
- Mac app now uses the One ROM liquid glass icon.
- Moved to libusb-less DFU implementation using `dfu-rs` and `nusb` crates.
- Moved to manual rescanning to detect probes and USB devices.
- Added network connectivity icon.
- Single universal macOS dmg installer instead of separate Intel and Apple Silicon versions.