// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use onerom_config::hw::Board;
use onerom_config::mcu::Variant;
use onerom_fw::{assemble_firmware, validate_sizes};

use crate::args;
use crate::firmware::{
    acquire_firmware, build_rom_image, parse_firmware, verify_assembled_firmware,
};
use crate::utils::{check_device, resolve_board};
use onerom_cli::device::select_device;
use onerom_cli::usb::{RebootMode, flash_program, flash_program_read, reboot};
use onerom_cli::{Error, Options};

fn validate_program_args(args: &args::program::ProgramArgs) -> Result<(), Error> {
    if args.msd && !args.stopped {
        return Err(Error::InvalidArgument(
            "--msd requires --stopped".to_string(),
        ));
    }

    if !args.rom.is_empty() && args.no_config {
        return Err(Error::InvalidArgument(
            "--no-config cannot be used with --rom".to_string(),
        ));
    }

    // Clap cannot express "this group is required unless --no-config is set",
    // so we enforce it here instead, with the group set to required(false)
    if !args.no_config
        && args.config.is_none()
        && args.rom.is_empty()
        && args.firmware.is_none()
        && args.base_firmware.is_none()
    {
        return Err(Error::NoFirmwareSource);
    }

    Ok(())
}

async fn verify_flash(options: &Options, data: &[u8]) -> Result<(), Error> {
    let device = options.device.as_ref().unwrap();
    if options.verbose {
        println!("Verifying {} bytes...", data.len());
    }

    let readback = flash_program_read(device, data.len() as u32).await?;

    for (i, (expected, actual)) in data.iter().zip(readback.iter()).enumerate() {
        if expected != actual {
            return Err(Error::VerifyFailed(i, *expected, *actual));
        }
    }

    println!("Verification passed");
    Ok(())
}

async fn acquire_program_image(
    options: &Options,
    args: &args::program::ProgramArgs,
    board: &Option<Board>,
    mcu: &Variant,
) -> Result<Vec<u8>, Error> {
    if let Some(firmware) = &args.firmware {
        if options.verbose {
            println!("Using pre-built firmware: {firmware}");
        }
        return std::fs::read(firmware).map_err(Error::from);
    }

    if let Some(path) = args.base_firmware.as_ref()
        && args.config.is_none()
        && args.rom.is_empty()
    {
        if options.verbose {
            println!("Flashing base firmware without ROM config: {path}");
        }
        return std::fs::read(path).map_err(Error::from);
    }

    // Build mode: acquire base firmware, build ROM image, assemble
    let board = board.as_ref().ok_or(Error::NoBoardOrDevice)?;
    let config = if let Some(config) = &args.config {
        if options.verbose {
            println!("Using ROM config: {config}");
        }
        Some(config.as_str())
    } else if args.no_config {
        if options.verbose {
            println!("No config file specified, proceeding without ROM images");
        }
        None
    } else {
        assert!(!args.rom.is_empty());
        return Err(Error::InvalidArgument(
            "--rom is not yet supported".to_string(),
        ));
    };

    let (firmware_data, version, _version_str) =
        acquire_firmware(options, &args.base_firmware, &args.version, board, mcu).await?;

    let (fw_props, metadata, image_data, desc) =
        build_rom_image(options, config, version, *board, *mcu).await?;

    validate_sizes(&fw_props, &firmware_data, &metadata, &image_data)?;

    if options.verbose && !desc.is_empty() {
        println!("ROM configuration:\n{desc}");
    }

    assemble_firmware(firmware_data, metadata, image_data).map_err(Into::into)
}

async fn inspect_firmware(options: &Options, data: &[u8]) -> Result<(), Error> {
    if !options.verbose {
        return Ok(());
    }

    match parse_firmware(data).await {
        Ok(info) => println!("Firmware version: {}", info.version),
        Err(e) => println!("Warning: could not parse firmware: {e}"),
    }
    Ok(())
}

fn write_firmware_file(path: &str, data: &[u8]) -> Result<(), Error> {
    std::fs::write(path, data)?;
    println!("Firmware written to {path}");
    Ok(())
}

async fn flash_device(options: &mut Options, data: &[u8]) -> Result<(), Error> {
    let device = options.device.as_ref().unwrap();

    if device.is_running() {
        if options.verbose {
            println!("Device is running, rebooting into stopped mode...");
        }
        let serial = device.serial.clone();
        reboot(device, RebootMode::Stopped { msd: false }).await?;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Re-enumerate — old device_info is stale after reboot
        let selector = serial.as_deref().unwrap_or("*");
        let new_device = select_device(Some(selector), true).await?;

        if new_device.is_running() {
            return Err(Error::DeviceRunning);
        }
        options.device = Some(new_device);
    }

    let device = options.device.as_ref().unwrap();
    if options.verbose {
        println!("Flashing {} bytes...", data.len());
    }

    flash_program(device, data).await?;

    Ok(())
}

async fn reboot_device(options: &Options, args: &args::program::ProgramArgs) -> Result<(), Error> {
    if args.no_reboot {
        println!("Skipping reboot (--no-reboot)");
        return Ok(());
    }

    let device = options.device.as_ref().unwrap();
    let mode = if args.stopped {
        RebootMode::Stopped { msd: args.msd }
    } else {
        RebootMode::Running
    };

    if options.verbose {
        println!("Rebooting device into {mode} mode...");
    }
    reboot(device, mode).await?;
    if options.verbose {
        println!("Rebooted device into {mode} mode");
    }
    Ok(())
}

pub async fn cmd_program(
    options: &mut Options,
    args: &args::program::ProgramArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    validate_program_args(args)?;

    let board = resolve_board(options, &args.board)?;
    let mcu = Variant::RP2350;

    let data = acquire_program_image(options, args, &board, &mcu).await?;
    verify_assembled_firmware(options, &data, args.force).await?;
    inspect_firmware(options, &data).await?;

    if let Some(out) = &args.output {
        write_firmware_file(out, &data)?;
    }

    println!("Programming device - DO NOT DISCONNECT");

    flash_device(options, &data).await?;

    if args.verify {
        verify_flash(options, &data).await?;
    }

    println!("Programming complete");

    reboot_device(options, args).await
}
