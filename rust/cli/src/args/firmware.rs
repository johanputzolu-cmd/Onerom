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
    /// Build a One ROM firmware binary from a ROM configuration.
    ///
    /// Produces a flashable firmware binary for the specified board and MCU.
    /// ROM images and configuration are supplied either via a JSON config
    /// file or individual --rom arguments.
    ///
    /// Examples:
    ///
    ///   onerom firmware build --config c64.json --board fire-24-e --out firmware.bin
    ///
    ///   onerom firmware build --board fire-24-e \
    ///       --rom image=kernal.bin,type=2364,cs=active_low \
    ///       --out firmware.bin
    Build(FirmwareBuildArgs),

    /// Inspect the contents of a One ROM firmware binary.
    ///
    /// Displays the firmware version, board type, MCU, and details of any
    /// embedded ROM images and metadata.
    ///
    /// Example:
    ///
    ///   onerom firmware inspect firmware.bin
    Inspect(FirmwareInspectArgs),

    /// List available One ROM firmware releases.
    ///
    /// Fetches the release manifest from the network and displays available
    /// firmware versions with their supported board types and MCUs.
    ///
    /// Example:
    ///
    ///   onerom firmware releases
    Releases(FirmwareReleasesArgs),

    /// Download a One ROM firmware binary from a release.
    ///
    /// Downloads the base (ROM-less) firmware binary for the specified
    /// version, board, and MCU.
    ///
    /// Use `program` to build and flash a complete firmware with ROM images in one step.
    ///
    /// Use `firmware build` to build a complete firmware with ROM images
    /// from a config, but without flashing.
    ///
    /// Example:
    ///
    ///   onerom firmware download --version 0.6.5 --board fire-24-e --out firmware.bin
    Download(FirmwareDownloadArgs),
}

#[derive(Debug, Args)]
#[command(group = clap::ArgGroup::new("config_source").required(false).args(["config", "rom"]))]
pub struct FirmwareBuildArgs {
    /// ROM configuration JSON file. Mutually exclusive with --rom.
    #[arg(long, value_name = "FILE", conflicts_with = "rom")]
    pub config: Option<String>,

    /// ROM image specification. May be repeated for multiple images.  (Not yet supported.)
    ///
    /// Format: image=<file>,type=<romtype>,cs=<csconfig>
    ///
    /// Example: --rom image=kernal.bin,type=2364,cs=active_low
    ///
    /// Mutually exclusive with --config.
    #[arg(long, value_name = "SPEC", conflicts_with = "config")]
    pub rom: Vec<String>,

    /// Target board type (e.g. fire-24-e). Required when not inferrable
    /// from a connected device.
    #[arg(long, short, value_name = "BOARD")]
    pub board: Option<String>,

    /// Firmware version to build against. Defaults to the latest release.
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Output file path. Defaults to onerom-<board>-<version>.bin.
    #[arg(
        long,
        short,
        visible_alias = "out",
        value_name = "FILE",
        conflicts_with = "path"
    )]
    pub output: Option<String>,

    /// Output directory. Uses the default filename within the given directory.
    #[arg(long, value_name = "DIR", conflicts_with = "output")]
    pub path: Option<String>,

    /// Use a local minimal firmware binary instead of downloading from the
    /// release server.
    ///
    /// This must be built with EXCLUDE_METADATA=1 and ROM_CONFIGS= in order to
    /// be suitable for then constructing a complete firmware image with this
    /// command.
    #[arg(long, value_name = "FILE", conflicts_with = "version")]
    pub base_firmware: Option<String>,

    /// Continue even if the assembled firmware has parse errors.
    #[arg(long, short)]
    pub force: bool,

    /// Confirm flashing a base firmware with no ROM configuration.
    ///
    /// Only needed when --base-firmware is used without --config or --rom.
    #[arg(long)]
    pub no_config: bool,
}

impl CommandTrait for FirmwareBuildArgs {
    fn requires_device(&self) -> bool {
        false
    }
}

#[derive(Debug, Args)]
pub struct FirmwareInspectArgs {
    /// Firmware binary file to inspect.
    #[arg(long, visible_alias = "fw", value_name = "FILE")]
    pub firmware: Option<String>,

    /// Inspect release firmware for this board type.
    #[arg(long, short, value_name = "BOARD", conflicts_with = "firmware")]
    pub board: Option<String>,

    /// Firmware version to inspect. Defaults to latest.
    #[arg(long, value_name = "VERSION", conflicts_with = "firmware")]
    pub version: Option<String>,
}

impl CommandTrait for FirmwareInspectArgs {
    fn requires_device(&self) -> bool {
        false
    }
}

#[derive(Debug, Args)]
pub struct FirmwareReleasesArgs {
    /// Show only releases for this board type.
    #[arg(long, short, value_name = "BOARD")]
    pub board: Option<String>,

    /// Show all releases, even if a device is attached and detected
    #[arg(long, short, conflicts_with = "board")]
    pub all: bool,
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
    ///
    /// Will be inferred from device if not included.
    #[arg(long, short, value_name = "BOARD")]
    pub board: Option<String>,

    /// Output file path. Defaults to onerom_<board>_<version>.bin.
    #[arg(
        long,
        short,
        visible_alias = "out",
        value_name = "FILE",
        conflicts_with = "path"
    )]
    pub output: Option<String>,

    /// Output directory. Uses the default filename within the given directory.
    #[arg(long, value_name = "DIR", conflicts_with = "output")]
    pub path: Option<String>,
}

impl CommandTrait for FirmwareDownloadArgs {
    fn requires_device(&self) -> bool {
        false
    }
}
