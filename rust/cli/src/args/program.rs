// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom program`.

use crate::args::CommandTrait;
use clap::Args;

// See Commands::Program in args/mod.rs for the top-level documentation of
// this command and examples.
#[derive(Debug, Args)]
pub struct ProgramArgs {
    /// ROM configuration JSON file. Mutually exclusive with --rom.
    #[arg(long, value_name = "FILE", conflicts_with = "rom")]
    pub config: Option<String>,

    /// ROM image specification. May be repeated for multiple images.
    ///
    /// Format: image=<file>,type=<romtype>,cs=<csconfig>
    ///
    /// Example: --rom image=kernal.bin,type=2364,cs=active_low
    ///
    /// Mutually exclusive with --config.
    #[arg(long, value_name = "SPEC", conflicts_with = "config")]
    pub rom: Vec<String>,

    /// Use a pre-built firmware binary instead of building from a config.
    ///
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

    /// Write the built firmware to this file in addition to flashing it.
    #[arg(long, short, value_name = "FILE")]
    pub out: Option<String>,

    /// After flashing, reboot the device into stopped mode, insead of
    /// running.
    #[arg(long, short)]
    pub stopped: bool,
}

impl CommandTrait for ProgramArgs {
    fn requires_device(&self) -> bool {
        true
    }
}
