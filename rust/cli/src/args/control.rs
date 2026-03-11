// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom control`.

use clap::{ArgGroup, Args, Subcommand, ValueEnum};

#[derive(Debug, Args)]
pub struct ControlArgs {
    #[command(subcommand)]
    pub command: ControlCommands,
}

#[derive(Debug, Subcommand)]
pub enum ControlCommands {
    /// Blink the status LED to identify a physical One ROM device (not yet supported).
    ///
    /// Useful when multiple One ROM devices are connected and you need to
    /// identify which physical device corresponds to a given serial number
    /// or board type.
    ///
    /// Example:
    ///   onerom control blink
    ///   onerom --device my-c64 control blink
    Blink(ControlBlinkArgs),

    /// Reboot the One ROM device.
    ///
    /// Restarts the One ROM firmware. The device will re-initialise and
    /// resume serving ROM images after the reboot.  This command pauses after
    /// a reboot to give the device time to re-enumerate
    ///
    /// Example:
    ///   onerom control reboot
    Reboot(ControlRebootArgs),

    /// Assert the host reset signal via the One ROM reset GPIO (not yet supported).
    ///
    /// Drives the reset pin to reset the host system the One ROM is
    /// installed in. Useful in scripted workflows to reset the host after
    /// programming a new ROM image.
    ///
    /// Example:
    ///   onerom control reset
    ///   onerom control reset --hold 500
    Reset(ControlResetArgs),

    /// Select the active ROM slot (not yet supported).
    ///
    /// Switches the device to serving the specified image slot. This takes
    /// effect immediately but does not persist across power cycles unless
    /// the device firmware is configured to do so.
    ///
    /// Example:
    ///   onerom control select --slot 2
    Select(ControlSelectArgs),

    /// Set the state of a One ROM GPIO pin (not yet supported).
    ///
    /// Sets the specified GPIO pin to high, low, or high-impedance (z).
    ///
    /// Example:
    ///   onerom control gpio --pin 3 --state high
    ///   onerom control gpio --pin 3 --state z
    Gpio(ControlGpioArgs),
}

#[derive(Debug, Args)]
pub struct ControlBlinkArgs {
    /// Duration in milliseconds to flash the LED. Defaults to 3000.
    #[arg(long, value_name = "MS", default_value = "3000")]
    pub duration: u32,
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
}

#[derive(Debug, Args)]
pub struct ControlResetArgs {
    /// Duration in milliseconds to hold the reset signal asserted.
    /// Defaults to 100.
    #[arg(long, value_name = "MS", default_value = "100")]
    pub hold: u32,
}

#[derive(Debug, Args)]
pub struct ControlSelectArgs {
    /// Image slot index to activate (0-15).
    #[arg(long, short, value_name = "INDEX", required = true)]
    pub slot: u8,
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
