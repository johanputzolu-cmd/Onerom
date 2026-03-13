// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom control`.

use crate::args::CommandTrait;
use crate::utils::{parse_u8, parse_u32};
use clap::{ArgGroup, Args, Subcommand, ValueEnum};
use enum_dispatch::enum_dispatch;

#[derive(Debug, Args)]
pub struct ControlArgs {
    #[command(subcommand)]
    pub command: ControlCommands,
}

impl CommandTrait for ControlArgs {
    fn requires_device(&self) -> bool {
        self.command.requires_device()
    }
}

#[enum_dispatch(CommandTrait)]
#[derive(Debug, Subcommand)]
pub enum ControlCommands {
    /// Blink the status LED to identify a physical One ROM device (not yet supported).
    ///
    /// Useful when multiple One ROM devices are connected and you need to
    /// identify which physical device corresponds to a given serial number
    /// or board type.
    ///
    /// Examples:
    ///
    ///   onerom control blink
    ///
    ///   onerom --device my-c64 control blink
    Blink(ControlBlinkArgs),

    /// Reboot the One ROM.
    ///
    /// Restarts the One ROM firmware. The device will re-initialise and
    /// resume serving ROM images after the reboot.
    ///
    /// By default, this command pauses after a reboot to give the device time
    /// to re-enumerate.
    ///
    /// Example:
    ///
    ///   onerom control reboot
    Reboot(ControlRebootArgs),

    /// Assert the host reset signal via the One ROM reset GPIO (not yet supported).
    ///
    /// Drives the reset pin to reset the host system the One ROM is
    /// installed in. Useful in scripted workflows to reset the host after
    /// programming a new ROM image.
    ///
    /// Examples:
    ///
    ///   onerom control reset
    ///
    ///   onerom control reset --hold 500
    Reset(ControlResetArgs),

    /// Select the active ROM slot (not yet supported).
    ///
    /// Switches the device to serving the specified image slot. This takes
    /// effect immediately but does not persist across power cycles unless
    /// the device firmware is configured to do so.
    ///
    /// Example:
    ///
    ///   onerom control select --slot 2
    Select(ControlSelectArgs),

    /// Set the state of a One ROM GPIO pin (not yet supported).
    ///
    /// Sets the specified GPIO pin to high, low, or high-impedance (z).
    ///
    /// Example:
    ///
    ///   onerom control gpio --pin 3 --state high
    ///
    ///   onerom control gpio --pin 3 --state z
    Gpio(ControlGpioArgs),

    /// Write data to One ROM's SRAM or the live ROM image.
    ///
    /// Poke provides transient (non-persistent) writes to device memory. Changes
    /// are lost on reboot. Use `update` subcommands for persistent flash writes.
    ///
    /// Data can be written as a single byte value or from a binary file.
    ///
    /// Example:
    ///
    ///   onerom control poke memory --address 0x20000010 --byte 0xFF
    ///
    ///   onerom control poke memory --address 0x20000010 --input patch.bin
    ///
    ///   onerom control poke live --address 0x100 --byte 0xEA
    ///
    ///   onerom control poke live --address 0x100 --input patch.bin
    #[command(
        subcommand_value_name = "COMMAND",
        subcommand_help_heading = "Commands"
    )]
    Poke(ControlPokeArgs),

    /// Erase this One ROM's flash memory.
    ///
    /// Permanently erases all flash contents on the device, including
    /// firmware, metadata and ROM images.
    ///
    /// Once a One ROM has been erased it will subsequently boot into the
    /// RP2350 bootloader from where it can be reprogrammed.
    ///
    /// Example:
    ///
    ///   onerom control erase
    Erase(ControlEraseArgs),
}

#[derive(Debug, Args)]
pub struct ControlBlinkArgs {
    /// Duration in milliseconds to flash the LED. Defaults to 3000.
    #[arg(long, value_name = "MS", default_value = "3000")]
    pub duration: u32,
}

impl CommandTrait for ControlBlinkArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Args)]
#[command(group = ArgGroup::new("reboot_mode").required(true).multiple(false))]
pub struct ControlRebootArgs {
    /// Reboot the device into stopped (bootloader) state
    #[arg(long, short, group = "reboot_mode")]
    pub stopped: bool,

    /// Reboot the device into running (byte serving) state
    #[arg(long, short, group = "reboot_mode")]
    pub running: bool,

    /// Don't pause after reboot for the device to re-enumerate
    #[arg(long, short)]
    pub fast: bool,

    /// Mount mass storage device when rebooting into stopped mode.
    #[arg(long, short = 'm', conflicts_with = "running")]
    pub msd: bool,
}

impl CommandTrait for ControlRebootArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Args)]
pub struct ControlResetArgs {
    /// Duration in milliseconds to hold the reset signal asserted.
    /// Defaults to 100.
    #[arg(long, value_name = "MS", default_value = "100")]
    pub hold: u32,
}

impl CommandTrait for ControlResetArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Args)]
pub struct ControlSelectArgs {
    /// Image slot index to activate (0-15).
    #[arg(long, short, value_name = "INDEX", required = true)]
    pub slot: u8,
}

impl CommandTrait for ControlSelectArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum GpioState {
    /// Drive the pin high.
    High,
    /// Drive the pin low.
    Low,
    /// Set the pin to high-impedance (tri-state).
    Z,
}

#[derive(Debug, Args)]
pub struct ControlGpioArgs {
    /// GPIO pin number to control.
    #[arg(long, value_name = "PIN", required = true)]
    pub pin: u8,

    /// Desired pin state: high, low, or z (high-impedance).
    #[arg(long, value_name = "STATE", required = true)]
    pub state: GpioState,
}

impl CommandTrait for ControlGpioArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Args)]
pub struct ControlPokeArgs {
    #[command(subcommand)]
    pub command: ControlPokeCommands,
}

impl CommandTrait for ControlPokeArgs {
    fn requires_device(&self) -> bool {
        self.command.requires_device()
    }
}

#[derive(Debug, Args)]
#[command(group = ArgGroup::new("erase_target").required(true).args(["all", "offset", "address"]))]
#[command(group = ArgGroup::new("reboot_mode").args(["reboot_stopped", "reboot_running"]))]
pub struct ControlEraseArgs {
    /// Erase all flash contents.
    #[arg(long, short)]
    pub all: bool,

    /// Erase at offset(s) relative to flash base (0x10000000).
    ///
    /// Must be 4096-aligned. Pair each with a --size.
    /// Can be repeated for multiple ranges.
    /// Mutually exclusive with --address.
    #[arg(long, short, value_name = "OFFSET", value_parser = parse_u32, action = clap::ArgAction::Append, conflicts_with = "address", requires = "size")]
    pub offset: Vec<u32>,

    /// Erase at absolute address(es).
    ///
    /// Must be 4096-aligned. Pair each with a --size.
    /// Can be repeated for multiple ranges.
    /// Mutually exclusive with --offset.
    #[arg(long, value_name = "ADDRESS", value_parser = parse_u32, action = clap::ArgAction::Append, conflicts_with = "offset", requires = "size")]
    pub address: Vec<u32>,

    /// Size of each range to erase (paired with --offset or --address).
    ///
    /// Must be 4096-aligned. Specify once per --offset/--address.
    #[arg(long, value_name = "SIZE", value_parser = parse_u32, action = clap::ArgAction::Append, conflicts_with = "all")]
    pub size: Vec<u32>,

    /// Reboot into stopped (bootloader) mode after erasing.
    #[arg(long, short = 's')]
    pub reboot_stopped: bool,

    /// Reboot into running mode after erasing.
    #[arg(long, short = 'r')]
    pub reboot_running: bool,

    /// Mount mass storage device when rebooting into stopped mode.
    #[arg(long, short = 'm', requires = "reboot_stopped")]
    pub msd: bool,
}

impl CommandTrait for ControlEraseArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[enum_dispatch(CommandTrait)]
#[derive(Debug, Subcommand)]
pub enum ControlPokeCommands {
    /// Write a single byte or binary file to One ROM's SRAM.
    ///
    /// Writes data directly to the device's SRAM at the specified address.
    /// This is a transient operation — changes are lost on reboot.
    ///
    /// The address must be a valid SRAM address. When writing a file, the
    /// entire file contents are written starting at the given address. When
    /// writing a single byte, only that byte is written.
    ///
    /// Note: writing to arbitrary SRAM addresses can corrupt firmware state.
    /// Use with caution.
    ///
    /// Example:
    ///   onerom control poke memory --address 0x20000010 --value 0xFF
    ///   onerom control poke memory --address 0x20000000 --input patch.bin
    Memory(ControlPokeMemoryArgs),

    /// Write a single byte or binary file to the live ROM image.
    ///
    /// Writes data to the ROM image currently being served by the device,
    /// at the specified logical ROM address (starting from 0). This is a
    /// transient operation — changes are lost on reboot.
    ///
    /// This is useful for patching a running ROM image without reflashing.
    /// The address is a logical ROM offset, not a physical memory address.
    ///
    /// Example:
    ///   onerom control poke live --address 0x100 --value 0xEA
    ///   onerom control poke live --address 0 --input patch.bin
    Live(ControlPokeLiveArgs),
}

#[derive(Debug, Args)]
#[command(group = ArgGroup::new("poke_source").required(true).multiple(false))]
pub struct ControlPokeMemoryArgs {
    /// Write to this memory address on the device.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "ADDRESS", value_parser = parse_u32)]
    pub address: u32,

    /// Write this single byte value.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    /// Mutually exclusive with --input.
    #[arg(long, short, value_name = "BYTE", value_parser = parse_u8, group = "poke_source")]
    pub byte: Option<u8>,

    /// Write the contents of this binary file.
    ///
    /// Mutually exclusive with --value.
    #[arg(
        long,
        short,
        visible_alias = "in",
        value_name = "FILE",
        group = "poke_source"
    )]
    pub input: Option<String>,
}

impl CommandTrait for ControlPokeMemoryArgs {
    fn requires_device(&self) -> bool {
        true
    }
}

#[derive(Debug, Args)]
#[command(group = ArgGroup::new("poke_source").required(true).multiple(false))]
pub struct ControlPokeLiveArgs {
    /// Write to this logical ROM address, starting from 0.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    #[arg(long, short, value_name = "ADDRESS", value_parser = parse_u32, default_value = "0")]
    pub address: u32,

    /// Write this single byte value.
    ///
    /// Accepts decimal and hexadecimal (0x prefix) formats.
    /// Mutually exclusive with --input.
    #[arg(long, short, value_name = "BYTE", value_parser = parse_u8, group = "poke_source")]
    pub byte: Option<u8>,

    /// Write the contents of this binary file.
    ///
    /// Mutually exclusive with --value.
    #[arg(
        long,
        short,
        visible_alias = "in",
        value_name = "FILE",
        group = "poke_source"
    )]
    pub input: Option<String>,

    /// Only write bytes that differ from the current device's ROM content.
    ///
    /// Requires --input.
    #[arg(long, requires = "input", visible_alias = "deltas")]
    pub delta: bool,

    /// Show what would be written without actually writing.
    ///
    /// Requires --delta.
    #[arg(long, requires = "delta", visible_alias = "dryrun")]
    pub dry_run: bool,
}

impl CommandTrait for ControlPokeLiveArgs {
    fn requires_device(&self) -> bool {
        true
    }
}
