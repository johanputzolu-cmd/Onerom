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
//! The --serial option is global and can be specified at any level to select
//! a specific One ROM device when multiple are connected.

pub mod control;
pub mod firmware;
pub mod inspect;
pub mod program;
pub mod scan;
pub mod update;

use clap::{Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use log::debug;
use onerom_cli::LogLevel;

use crate::utils::parse_u16_hex_only;
use onerom_cli::{Error, Options};

use control::{
    ControlArgs, ControlCommands, ControlEraseArgs, ControlGpioArgs, ControlLedArgs,
    ControlLedBeaconArgs, ControlLedCommands, ControlLedFlameArgs, ControlLedOffArgs,
    ControlLedOnArgs, ControlPokeArgs, ControlPokeCommands, ControlPokeLiveArgs,
    ControlPokeMemoryArgs, ControlRebootArgs, ControlResetArgs, ControlSelectArgs,
};
use firmware::{
    FirmwareArgs, FirmwareBuildArgs, FirmwareCommands, FirmwareDownloadArgs, FirmwareInspectArgs,
    FirmwareReleasesArgs,
};
use inspect::{
    InspectArgs, InspectCommands, InspectGpioArgs, InspectImageArgs, InspectInfoArgs,
    InspectPeekArgs, InspectPeekCommands, InspectPeekLiveArgs, InspectPeekMemoryArgs,
    InspectSlotsArgs, InspectTelemetryArgs,
};
use program::ProgramArgs;
use scan::ScanArgs;
use update::{
    UpdateArgs, UpdateCommands, UpdateCommitArgs, UpdateOtpArgs, UpdateRenameArgs, UpdateSlotArgs,
};

#[enum_dispatch]
pub trait CommandTrait {
    fn requires_device(&self) -> bool;
}

/// Command line interface for One ROM - the most flexible retrom ROM replacement.
///
/// https://onerom.org/
///
/// Copyright (c) 2026 Piers Finlayson <piers@piers.rocks>
///
/// Manage One ROM devices, firmware, and ROM images. Run `onerom help <command>`
/// for detailed information on any subcommand.
///
/// Use `onerom scan` to discover connected devices before running device
/// commands.
#[derive(Debug, Parser)]
#[command(name = "onerom", version = concat!("v", env!("CARGO_PKG_VERSION")), about, long_about)]
pub struct Cli {
    /// Select a specific One ROM device by serial number.
    ///
    /// Required when multiple devices are connected.
    ///
    /// Accepts * and ? wildcards for partial matching.
    ///
    /// If omitted and exactly one device is connected, that device is used automatically.
    #[arg(global = true, long, short, value_name = "DEVICE")]
    pub serial: Option<String>,

    /// USB vendor/product ID pair (hex, e.g. 1234:abcd).
    ///
    /// Used to detect One ROMs using non-standard USB vendor/product IDs.  If
    /// specified, only those VID/PID pairs specified will be matched.
    ///
    /// Specify multiple pairs by specifying the --vid-pid argument multiple
    /// times.
    ///
    /// Use in conjunction with --unrecognised to manage devices that do not
    /// have a known One ROM firmware signature, such as unprogrammed or
    /// bricked devices.
    #[arg(global = true, long, short='i', visible_alias="id", value_name = "VID:PID", value_parser = parse_vid_pid, action = clap::ArgAction::Append)]
    pub vid_pid: Vec<(u16, u16)>,

    /// Allow management of unrecognised RP2350 devices and unprogrammed One ROMs.
    ///
    /// This is a global flag that can be used with any command to allow
    /// this tool to manage RP2350 devices that do not have a known One ROM
    /// firmware signature, such as unprogrammed or bricked devices.
    ///
    /// Note that even unrecognised devices must expose a valid picoboot USB
    /// interface to be detected and managed by this tool.
    ///
    /// Use with caution as this allows programming of any non-One ROM RP2350
    /// devices that are attached.
    ///
    /// Use in conjunction with --vid-pid to manage devices that have
    /// unexpected USB vendor and/or product IDs.
    #[arg(global = true, visible_alias = "unrecognized", long, short)]
    pub unrecognised: bool,

    /// Auto-confirm all prompts with "yes".
    ///
    /// This is a global flag that can be used with any command to
    /// automatically answer "yes" to all prompts, allowing for non-interactive
    /// use.
    ///
    /// Use with caution, as it may lead to unintended consequences if used
    /// without fully understanding the implications of the command being
    /// run.
    #[arg(global = true, long, short)]
    pub yes: bool,

    /// Enable verbose output.
    #[arg(global = true, long, short)]
    pub verbose: bool,

    /// Set logging level.
    #[arg(global = true, long, value_enum, default_value_t = LogLevel::Warn)]
    pub log_level: LogLevel,

    #[command(subcommand)]
    pub command: Commands,
}

fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let (vid, pid) = s
        .split_once(':')
        .ok_or_else(|| format!("expected VID:PID, got '{s}'"))?;
    let vid = parse_u16_hex_only(vid).map_err(|e| format!("invalid VID '{vid}': {e}"))?;
    let pid = parse_u16_hex_only(pid).map_err(|e| format!("invalid PID '{pid}': {e}"))?;
    Ok((vid, pid))
}

fn check_vid_pid_unique(vid_pid_list: &[(u16, u16)]) -> Result<(), Error> {
    let mut seen = std::collections::HashSet::new();
    for (vid, pid) in vid_pid_list {
        if !seen.insert((*vid, *pid)) {
            return Err(Error::InvalidArgument(format!(
                "duplicate VID:PID pair '{:04x}:{:04x}'",
                vid, pid
            )));
        }
    }
    Ok(())
}

impl Cli {
    pub async fn try_into_options(&mut self) -> Result<Options, Error> {
        // Build the options struct first.
        let mut options = Options {
            log_level: self.log_level.clone(),
            verbose: self.verbose,
            yes: self.yes,
            unrecognised: self.unrecognised,
            device: None,
            vid_pid: self.vid_pid.clone(),
        };

        // Check for duplicate VID/PID pairs
        check_vid_pid_unique(&options.vid_pid)?;

        let requires_device = self.command.requires_device();

        // Check if command needs a device
        if let Some(device) = self.serial.as_ref()
            && !requires_device
        {
            debug!("Device {device} specified but not required, retrieving it anyway");
        }

        // If a serial was specified, select it and add it to the options,
        // unless handling a scan command (in which case we're scanning for
        // all devices that meet some criteria).
        if let Commands::Scan(scan) = &mut self.command {
            // Save off the serial for special handling in the scan case.
            scan.serial = self.serial.clone();
        } else if let Some(serial) = self.serial.as_ref() {
            if options.verbose {
                println!("Scanning for device with serial '{}' ...", serial);
            }
            let device = onerom_cli::device::select_device(
                Some(serial),
                options.unrecognised,
                &options.vid_pid,
            )
            .await?;
            if options.verbose {
                println!("Found device: {device}");
            }
            options.device = Some(device);
        }

        // If no device was specified, attempt to detect one
        if options.device.is_none() && requires_device {
            if options.verbose {
                println!("No device specified, scanning for connected devices ...");
            }
            let device =
                onerom_cli::device::select_device(None, options.unrecognised, &options.vid_pid)
                    .await?;
            if options.verbose {
                println!("Found device: {device}");
            }
            options.device = Some(device);
        }

        Ok(options)
    }
}

#[enum_dispatch(CommandTrait)]
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Discover and list connected One ROM devices.
    Scan(ScanArgs),

    /// Build and flash One ROM firmware to a connected device.
    ///
    /// This is the primary workflow for most users. The board and MCU type are
    /// inferred from the connected device if not specified explicitly.
    ///
    /// With a single device connected and a config file:
    ///
    ///   onerom program --config c64.json
    ///
    /// With multiple devices connected, using a wildcard to select the target
    /// device:
    ///
    ///   onerom program --serial '5*' --config c64.json
    ///
    /// With explicit ROM arguments instead of a config file:
    ///
    ///   onerom program --board fire-24-e \
    ///       --rom image=kernal.bin,type=2364,cs=active_low \
    ///       --rom image=basic.bin,type=2364,cs=active_low
    ///
    /// Using a local, pre-built firmware binary, containing the ROM metadata and
    /// images:
    ///
    ///   onerom program --firmware firmware.bin
    ///
    /// Using a local, pre-built minimal firmware, with no ROM metadata or images,
    /// and specifying the ROMs via arguments:
    ///
    ///   onerom program --firmware minimal.bin \
    ///       --rom image=kernal.bin,type=2364,cs=active_low \
    ///       --rom image=basic.bin,type=2364,cs=active_low
    ///
    /// To save the firmware to file, **as well** as programming the device, use
    /// --out.
    ///
    ///   onerom program --config c64.json --out firmware.bin
    ///
    /// To generate a firmware binary without programming a device, use the
    /// 'firmware' command.
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
    /// across power cycles.
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

    /// Read data from One ROM's live ROM image.
    ///
    /// Top-level alias for `inspect peek live`. See `onerom inspect peek live --help`
    /// for full documentation.
    ///
    /// Example:
    ///
    ///   onerom peek live --address 0x100 --length 64
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Peek(InspectPeekLiveArgs),

    /// Write data to One ROM's live ROM image.
    ///
    /// Top-level alias for `control poke live`. See `onerom control poke live --help`
    /// for full documentation.
    ///
    /// Example:
    ///
    ///   onerom poke live --address 0x100 --input patch.bin
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Poke(ControlPokeLiveArgs),

    /// Reboot a One ROM.
    ///
    /// Restarts a selected One ROM device. The device re-initialises and
    /// resumes serving ROM images after the reboot.
    ///
    /// By default, this command briefly pauses after a reboot to give the
    /// device time to re-enumerate.
    ///
    /// Example:
    ///
    ///   onerom reboot
    Reboot(ControlRebootArgs),

    /// Build, inspect, and manage One ROM firmware binaries.
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Firmware(FirmwareArgs),
}
