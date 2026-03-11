// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom inspect`.

use crate::utils::parse_u32;
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[command(subcommand)]
    pub command: InspectCommands,
}

#[derive(Debug, Subcommand)]
pub enum InspectCommands {
    /// Display identity and configuration information for a One ROM device.
    ///
    /// Shows the device's serial number, user-assigned name, board type,
    /// MCU, firmware version, and hardware revision.
    ///
    /// Example:
    ///   onerom inspect info
    ///   onerom --device my-c64 inspect info
    Info(InspectInfoArgs),

    /// Display runtime telemetry from a One ROM device (not yet supported).
    ///
    /// Shows access counts, timing statistics, and other runtime metrics
    /// collected by the device firmware.
    ///
    /// Example:
    ///   onerom inspect telemetry
    Telemetry(InspectTelemetryArgs),

    /// List the ROM image slots stored on a One ROM device.
    ///
    /// Displays the index, ROM type, size, and description of each
    /// configured image slot, and indicates which slot is currently active.
    ///
    /// Example:
    ///   onerom inspect slots
    Slots(InspectSlotsArgs),

    /// Read and display the ROM image currently loaded in a slot (not yet supported).
    ///
    /// Displays or saves the ROM image data from the specified slot.
    /// If no slot is specified, reads the image currently being served.
    ///
    /// Example:
    ///   onerom inspect image --slot 2
    ///   onerom inspect image --slot 2 --out kernal-backup.bin
    Image(InspectImageArgs),

    /// Read and display the live ROM image.
    ///
    /// Can be used to read what byte One ROM will serve if queried for a
    /// particular address. This is a live read of the currently active image.
    ///
    /// Example:
    ///   onerom inspect live --address 0x100 --length 64
    ///   onerom inspect live --address 0 --length 8192 --out rom-image.bin
    Live(InspectLiveArgs),

    /// Read and display One ROM's SRAM and flash contents.
    ///
    /// Can be used to read the flash and SRAM from a One ROM device.  Note
    /// that when used on a device in the "Stopped" state, SRAM will not
    /// contain meaningful information.
    ///
    /// Most address that can be queried via the PICOBOOT protocol can be
    /// queried.  When in "Stopped" state, flash reads must be performed
    /// aligned to flash page boundaries.
    ///
    /// Example:
    ///   onerom inspect memory --address 0x20000000 --length 128
    ///   onerom inspect memory --address 0x10000000 --length 8192 --out flash-start.bin
    Memory(InspectMemoryArgs),

    /// Read the current state of the One ROM GPIO pins (not yet supported).
    ///
    /// Displays the direction and logic level of each exposed GPIO pin.
    ///
    /// Example:
    ///   onerom inspect gpio
    Gpio(InspectGpioArgs),
}

#[derive(Debug, Args)]
pub struct InspectInfoArgs {}

#[derive(Debug, Args)]
pub struct InspectTelemetryArgs {
    /// Output telemetry in JSON format.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InspectSlotsArgs {}

#[derive(Debug, Args)]
pub struct InspectImageArgs {
    /// Slot index to read (0-15). Reads the currently active slot if omitted.
    #[arg(long, short, value_name = "INDEX", value_parser = parse_u32)]
    pub slot: Option<u8>,

    /// Save the image data to this file.
    #[arg(long, short, value_name = "FILE", value_parser = parse_u32)]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct InspectLiveArgs {
    /// Read from the ROM image at this logic address, which starts from 0.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "Address", value_parser = parse_u32)]
    pub address: u32,

    /// Read this many bytes of data from the ROM image.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "LENGTH", value_parser = parse_u32)]
    pub length: u32,

    /// Save the image data to this file.
    #[arg(long, short, value_name = "FILE")]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct InspectMemoryArgs {
    /// Read from the ROM image at this logic address, which starts from 0.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "Address", value_parser = parse_u32)]
    pub address: u32,

    /// Read this many bytes of data from the ROM image
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "LENGTH", value_parser = parse_u32)]
    pub length: u32,

    /// Save the image data to this file.
    #[arg(long, short, value_name = "FILE")]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct InspectGpioArgs {
    /// Show only this specific pin.
    #[arg(long, value_name = "PIN")]
    pub pin: Option<u8>,
}
