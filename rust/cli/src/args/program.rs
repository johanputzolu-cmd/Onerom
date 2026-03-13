// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom program`.

use crate::args::CommandTrait;
use clap::Args;
use onerom_cli::usb::RebootArgs;

// See Commands::Program in args/mod.rs for the top-level documentation of
// this command and examples.
#[derive(Debug, Args)]
#[command(group = clap::ArgGroup::new("source").required(false).multiple(true).args(["config", "rom", "firmware", "base_firmware"]))]
pub struct ProgramArgs {
    /// ROM configuration JSON file. Mutually exclusive with --rom and --firmware.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["rom", "firmware"])]
    pub config: Option<String>,

    /// ROM image specification. May be repeated for multiple images.
    ///
    /// Format: image=<file>,type=<romtype>,cs=<csconfig>
    ///
    /// Example: --rom image=kernal.bin,type=2364,cs=active_low
    ///
    /// Mutually exclusive with --config and --firmware.
    #[arg(long, value_name = "SPEC", conflicts_with_all = ["config", "firmware"])]
    pub rom: Vec<String>,

    /// Flash a pre-built complete firmware binary directly.
    ///
    /// Mutually exclusive with --config, --rom, --base-firmware, and --version.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["config", "rom", "base_firmware", "version"])]
    pub firmware: Option<String>,

    /// Use a local minimal firmware binary instead of downloading from the
    /// release server.
    ///
    /// When used with --config or --rom, the ROM images are built into this
    /// firmware. When used alone, requires --no-config to confirm flashing
    /// without ROM images.
    ///
    /// Must be built with EXCLUDE_METADATA=1 and ROM_CONFIGS= in order to
    /// be suitable.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["firmware", "version"])]
    pub base_firmware: Option<String>,

    /// Confirm flashing a base firmware with no ROM configuration.
    ///
    /// Only needed when --base-firmware is used without --config or --rom.
    #[arg(long, conflicts_with_all = ["config", "rom", "firmware"])]
    pub no_config: bool,

    /// Target board type (e.g. fire-24-e). Inferred from connected device
    /// if not specified.
    #[arg(long, short, value_name = "BOARD")]
    pub board: Option<String>,

    /// Firmware version to build against. Defaults to the latest release.
    #[arg(long, value_name = "VERSION", conflicts_with_all = ["firmware", "base_firmware"])]
    pub version: Option<String>,

    /// Write the built firmware to this file in addition to flashing it.
    #[arg(long, short = 'p', value_name = "FILE")]
    pub output: Option<String>,

    /// After flashing, reboot the device into stopped mode instead of running.
    #[arg(long, short)]
    pub stopped: bool,

    /// Do not reboot the device after flashing.
    #[arg(long, conflicts_with = "stopped")]
    pub no_reboot: bool,

    /// Verify flash contents after programming by reading back. (Not yet supported.)
    #[arg(long)]
    pub verify: bool,

    /// Continue even if the assembled firmware has parse errors.
    #[arg(long, short)]
    pub force: bool,

    /// Mount mass storage device when rebooting into stopped mode.
    #[arg(long, short = 'm')]
    pub msd: bool,

    /// Don't pause after final reboot for the device to re-enumerate
    #[arg(long, conflicts_with = "no_reboot")]
    pub fast: bool,
}

impl CommandTrait for ProgramArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

impl From<&ProgramArgs> for RebootArgs {
    fn from(args: &ProgramArgs) -> Self {
        if args.no_reboot {
            RebootArgs::none()
        } else if args.stopped {
            RebootArgs::stopped(args.msd, args.fast)
        } else {
            RebootArgs::running(args.fast)
        }
    }
}
