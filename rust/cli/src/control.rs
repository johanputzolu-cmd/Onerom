// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::{args, utils};
use onerom_cli::usb::{RebootMode, reboot};
use onerom_cli::{Error, Options};

pub async fn cmd_blink(
    options: &Options,
    _args: &args::control::ControlBlinkArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control blink".to_string()))
}

pub async fn cmd_reboot(
    options: &Options,
    args: &args::control::ControlRebootArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let device = options.device.as_ref().unwrap();
    assert!(
        !(args.stopped && args.running),
        "Cannot specify both --stopped and --running"
    );
    let mode = if args.stopped {
        RebootMode::Stopped
    } else {
        RebootMode::Running
    };
    if options.verbose {
        println!("Rebooting device: {device}");
    } else {
        println!("Rebooting device")
    }
    reboot(device, mode).await?;
    println!("Rebooted device into {mode} mode");

    // Wait a few seconds for the device to re-enumerate before returning
    if !args.fast {
        println!("Pausing for device to re-enumerate");
        smol::Timer::after(std::time::Duration::from_secs(5)).await;
    }

    Ok(())
}

pub async fn cmd_reset(
    options: &Options,
    _args: &args::control::ControlResetArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control reset".to_string()))
}

pub async fn cmd_select(
    options: &Options,
    _args: &args::control::ControlSelectArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control select".to_string()))
}

pub async fn cmd_gpio(
    options: &Options,
    _args: &args::control::ControlGpioArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control gpio".to_string()))
}
