// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom firmware`.

use crate::args::CommandTrait;
use clap::{Args, Subcommand};
use enum_dispatch::enum_dispatch;

#[derive(Debug, Args)]
pub struct FirmwareArgs {
    #[command(subcommand)]
    pub command: FirmwareCommands,
}

impl CommandTrait for FirmwareArgs {
    fn requires_device(&self) -> bool {
        self.command.requires_device()
    }
}

#[enum_dispatch(CommandTrait)]
#[derive(Debug, Subcommand)]
pub enum FirmwareCommands {
    /// Build a One ROM firmware binary from a ROM configuration (not yet supported).
    ///
    /// Produces a flashable firmware binary for the specified board and MCU.
    /// ROM images and configuration are supplied either via a JSON config
    /// file or individual --rom arguments.
    ///
    /// Examples:
    ///   onerom firmware build --config c64.json --board fire-24-e --out firmware.bin
    ///   onerom firmware build --board fire-24-e --mcu rp2350 \
    ///       --rom image=kernal.bin,type=2364,cs=active_low \
    ///       --out firmware.bin
    Build(FirmwareBuildArgs),

    /// Inspect the contents of a One ROM firmware binary (not yet supported).
    ///
    /// Displays the firmware version, board type, MCU, and details of any
    /// embedded ROM images and metadata.
    ///
    /// Example:
    ///   onerom firmware inspect firmware.bin
    Inspect(FirmwareInspectArgs),

    /// List available One ROM firmware releases.
    ///
    /// Fetches the release manifest from the network and displays available
    /// firmware versions with their supported board types and MCUs.
    ///
    /// Example:
    ///   onerom firmware releases
    Releases(FirmwareReleasesArgs),

    /// Download a One ROM firmware binary from a release (not yet supported).
    ///
    /// Downloads the base (ROM-less) firmware binary for the specified
    /// version, board, and MCU. Use `onerom program` to build and flash
    /// a complete firmware with ROM images in one step.
    ///
    /// Example:
    ///   onerom firmware download --version 0.6.5 --board fire-24-e --out firmware.bin
    Download(FirmwareDownloadArgs),
}

#[derive(Debug, Args)]
pub struct FirmwareBuildArgs {
    /// ROM configuration JSON file. Mutually exclusive with --rom.
    #[arg(long, value_name = "FILE", conflicts_with = "rom")]
    pub config: Option<String>,

    /// ROM image specification. May be repeated for multiple images.
    /// Format: image=<file>,type=<romtype>,cs=<csconfig>
    /// Example: --rom image=kernal.bin,type=2364,cs=active_low
    /// Mutually exclusive with --config.
    #[arg(long, value_name = "SPEC", conflicts_with = "config")]
    pub rom: Vec<String>,

    /// Target board type (e.g. fire-24-e). Required when not inferrable
    /// from a connected device.
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,

    /// Target MCU variant (e.g. rp2350). Required when not inferrable
    /// from a connected device.
    #[arg(long, value_name = "MCU")]
    pub mcu: Option<String>,

    /// Firmware version to build against. Defaults to the latest release.
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Path for the output firmware binary.
    #[arg(long, short, value_name = "FILE", required = true)]
    pub out: String,
}

impl CommandTrait for FirmwareBuildArgs {
    fn requires_device(&self) -> bool {
        false
    }
}

#[derive(Debug, Args)]
pub struct FirmwareInspectArgs {
    /// Firmware binary file to inspect.
    #[arg(value_name = "FILE")]
    pub file: String,
}

impl CommandTrait for FirmwareInspectArgs {
    fn requires_device(&self) -> bool {
        false
    }
}

#[derive(Debug, Args)]
pub struct FirmwareReleasesArgs {
    /// Show only releases for this board type.
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,
}

impl CommandTrait for FirmwareReleasesArgs {
    fn requires_device(&self) -> bool {
        false
    }
}

#[derive(Debug, Args)]
pub struct FirmwareDownloadArgs {
    /// Firmware version to download (e.g. 0.6.5). Defaults to latest.
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Target board type (e.g. fire-24-e).
    #[arg(long, value_name = "BOARD", required = true)]
    pub board: String,

    /// Target MCU variant (e.g. rp2350).
    #[arg(long, value_name = "MCU", required = true)]
    pub mcu: String,

    /// Output file path. Defaults to onerom-<board>-<mcu>-<version>.bin.
    #[arg(long, short, value_name = "FILE")]
    pub out: Option<String>,
}

impl CommandTrait for FirmwareDownloadArgs {
    fn requires_device(&self) -> bool {
        false
    }
}
