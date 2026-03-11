// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! CLI argument definitions for the One ROM command-line interface.
//!
//! The top-level structure is:
//!   onerom scan                  - Discover connected One ROM devices
//!   onerom firmware <subcommand> - Firmware binary management
//!   onerom program               - Build and flash firmware to a device
//!   onerom inspect <subcommand>  - Read-only device state and information
//!   onerom control <subcommand>  - Transient device actions
//!   onerom update <subcommand>   - Persistent device modifications
//!
//! The --device option is global and can be specified at any level to select
//! a specific One ROM device when multiple are connected.

pub mod control;
pub mod firmware;
pub mod inspect;
pub mod program;
pub mod scan;
pub mod update;

use clap::{Parser, Subcommand};

use onerom_cli::{Error, Options};

use control::ControlArgs;
use firmware::FirmwareArgs;
use inspect::InspectArgs;
use program::ProgramArgs;
use scan::ScanArgs;
use update::UpdateArgs;

/// One ROM command-line interface.
///
/// Manage One ROM devices, firmware, and ROM images. Run `onerom help <command>`
/// for detailed information on any subcommand.
///
/// Use `onerom scan` to discover connected devices before running device
/// commands.
#[derive(Debug, Parser)]
#[command(name = "onerom", version = concat!("v", env!("CARGO_PKG_VERSION")), about, long_about = None)]
pub struct Cli {
    /// Select a specific One ROM device by serial number.  Required when
    /// multiple devices are connected. If omitted and exactly one device is
    /// connected, that device is used automatically.
    #[arg(global = true, long, short, value_name = "DEVICE")]
    pub device: Option<String>,

    /// Enable verbose output.
    #[arg(global = true, long, short)]
    pub verbose: bool,

    /// Enable debug logging output.
    #[arg(global = true, long)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub async fn try_into_options(&self) -> Result<Options, Error> {
        let mut options = Options {
            debug: self.debug,
            verbose: self.verbose,
            device: None,
        };

        if let Some(device) = self.device.as_ref() {
            if options.verbose {
                println!("Scanning for device '{}' ...", device);
            }
            let device = onerom_cli::device::select_device(Some(device)).await?;
            if options.verbose {
                println!("Found device: {device}");
            }
            options.device = Some(device);
        }

        Ok(options)
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Discover and list connected One ROM devices.
    Scan(ScanArgs),

    /// Build, inspect, and manage One ROM firmware binaries.
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Firmware(FirmwareArgs),

    /// Build and flash One ROM firmware in a single operation (not yet supported).
    ///
    /// This is the primary workflow for most users. The board and MCU type
    /// are inferred from the connected device if not specified. With a JSON
    /// config file or explicit --rom arguments, the firmware is built and
    /// flashed in one step.
    Program(ProgramArgs),

    /// Read-only inspection of a connected One ROM device.
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Inspect(InspectArgs),

    /// Perform transient actions on a connected One ROM device.
    ///
    /// These actions affect the device's current state but do not persist
    /// across power cycles, with the exception of operations that explicitly
    /// write to flash.
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Control(ControlArgs),

    /// Make persistent modifications to a connected One ROM device.
    ///
    /// These operations write to the device's flash memory and survive
    /// power cycles.
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Update(UpdateArgs),
}
