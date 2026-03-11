// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! One ROM CLI library.
//!
//! Implements the logic behind each CLI command. The binary in main.rs
//! handles argument parsing and output formatting; this library owns
//! everything in between.

pub mod device;
pub mod error;
pub mod scan;
pub mod usb;

pub use device::{Device, DeviceState};
pub use error::Error;

pub struct Options {
    pub verbose: bool,
    pub debug: bool,
    pub device: Option<Device>,
}
