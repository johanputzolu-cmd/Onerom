// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::{
    args,
    utils::{check_device, check_live_read_write},
};
use onerom_cli::usb::{RebootMode, reboot, write_memory};
use onerom_cli::{Error, Options};

pub async fn cmd_blink(
    options: &Options,
    args: &args::control::ControlBlinkArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control blink".to_string()))
}

pub async fn cmd_reboot(
    options: &Options,
    args: &args::control::ControlRebootArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
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
    args: &args::control::ControlResetArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control reset".to_string()))
}

pub async fn cmd_select(
    options: &Options,
    args: &args::control::ControlSelectArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control select".to_string()))
}

pub async fn cmd_gpio(
    options: &Options,
    args: &args::control::ControlGpioArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("control gpio".to_string()))
}

// Resolve poke input — either a single byte value or the contents of a file.
//
// The ArgGroup on the args structs guarantees exactly one of these is Some.
fn poke_data(value: Option<u8>, input: Option<&String>) -> Result<Vec<u8>, Error> {
    if let Some(byte) = value {
        Ok(vec![byte])
    } else if let Some(path) = input {
        std::fs::read(path).map_err(|e| Error::Other(e.to_string()))
    } else {
        // Clap ArgGroup ensures this is unreachable, but be explicit
        Err(Error::Other("No data source specified".to_string()))
    }
}

pub async fn cmd_poke_memory(
    options: &Options,
    args: &args::control::ControlPokeMemoryArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    let device = options.device.as_ref().unwrap();

    let data = poke_data(args.byte, args.input.as_ref())?;
    write_memory(device, args.address, &data).await?;

    if options.verbose {
        println!("Wrote {} byte(s) to 0x{:08x}", data.len(), args.address);
    }

    Ok(())
}

pub async fn cmd_poke_live(
    options: &Options,
    args: &args::control::ControlPokeLiveArgs,
) -> Result<(), Error> {
    let data = poke_data(args.byte, args.input.as_ref())?;
    let address = check_live_read_write(options, args.address, data.len() as u32, args)?;

    let device = options.device.as_ref().unwrap();
    write_memory(device, address, &data).await?;

    if options.verbose {
        println!(
            "Wrote {} byte(s) to live ROM offset 0x{:08x}",
            data.len(),
            args.address
        );
    }

    Ok(())
}
