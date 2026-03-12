// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom program`.

use crate::args::CommandTrait;
use clap::Args;

/// Build and flash One ROM firmware to a connected device.
///
/// This is the primary workflow for most users. The board and MCU type are
/// inferred from the connected device if not specified explicitly.
///
/// With a single device connected and a config file:
///   onerom program --config c64.json
///
/// With multiple devices connected:
///   onerom program --device my-c64 --config c64.json
///
/// With explicit ROM arguments instead of a config file:
///   onerom program --board fire-24-e \
///       --rom image=kernal.bin,type=2364,cs=active_low \
///       --rom image=basic.bin,type=2364,cs=active_low
///
/// Using a pre-built firmware binary:
///   onerom program --firmware firmware.bin
///
/// With no device connected, --out is required and the firmware is written
/// to a file rather than flashed:
///   onerom program --config c64.json --out firmware.bin
#[derive(Debug, Args)]
pub struct ProgramArgs {
    /// ROM configuration JSON file. Mutually exclusive with --rom.
    #[arg(long, value_name = "FILE", conflicts_with = "rom")]
    pub config: Option<String>,

    /// ROM image specification. May be repeated for multiple images.
    /// Format: image=<file>,type=<romtype>,cs=<csconfig>
    /// Example: --rom image=kernal.bin,type=2364,cs=active_low
    /// Mutually exclusive with --config.
    #[arg(long, value_name = "SPEC", conflicts_with = "config")]
    pub rom: Vec<String>,

    /// Use a pre-built firmware binary instead of building from a config.
    /// Mutually exclusive with --config and --rom.
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with_all = ["config", "rom"]
    )]
    pub firmware: Option<String>,

    /// Target board type (e.g. fire-24-e). Inferred from connected device
    /// if not specified.
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,

    /// Target MCU variant (e.g. rp2350). Inferred from connected device
    /// if not specified.
    #[arg(long, value_name = "MCU")]
    pub mcu: Option<String>,

    /// Firmware version to build against. Defaults to the latest release.
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Write the built firmware to this file instead of (or in addition to)
    /// flashing it. Required when no device is connected.
    #[arg(long, short, value_name = "FILE")]
    pub out: Option<String>,
}

impl CommandTrait for ProgramArgs {
    fn requires_device(&self) -> bool {
        true
    }
}
