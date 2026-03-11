// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::args::inspect::{
    InspectGpioArgs, InspectImageArgs, InspectInfoArgs, InspectLiveArgs, InspectMemoryArgs,
    InspectSlotsArgs, InspectTelemetryArgs,
};
use crate::utils;
use onerom_cli::usb::read_memory;
use onerom_cli::{Device, DeviceState, Error, Options};
use sdrr_fw_parser::SdrrCsState;

pub async fn cmd_info(options: &Options, _args: &InspectInfoArgs) -> Result<(), Error> {
    // Print the device summary
    utils::check_device(options)?;
    let device = options.device.as_ref().unwrap();

    println!("{device}");

    // Print the detailed device information as JSON if available
    if let Some(onerom) = device.onerom.as_ref() {
        if let Some(info) = onerom.flash.as_ref() {
            let json =
                serde_json::to_string_pretty(info).map_err(|e| Error::Other(e.to_string()))?;
            println!("Flash information:");
            println!("{json}");
        }
        if let Some(info) = onerom.ram.as_ref() {
            let json =
                serde_json::to_string_pretty(info).map_err(|e| Error::Other(e.to_string()))?;
            println!("Runtime information:");
            println!("{json}");
        }
    }

    Ok(())
}

pub async fn cmd_telemetry(options: &Options, _args: &InspectTelemetryArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("inspect telemetry".into()))
}

pub async fn cmd_slots(options: &Options, _args: &InspectSlotsArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let device = options.device.as_ref().unwrap();

    println!("{device}");
    if let Some(onerom) = device.onerom.as_ref()
        && let Some(info) = onerom.flash.as_ref()
    {
        let verbose = options.verbose;
        let set_count = info.rom_set_count;
        println!(
            "Configured with {set_count} slot{}:",
            if set_count == 1 { "" } else { "s" }
        );
        for (i, set) in info.rom_sets.iter().enumerate() {
            println!("  Slot {i}:");
            let set_location = set.data_ptr;
            let set_image_size = set.size;
            for (j, rom) in set.roms.iter().enumerate() {
                let mut cs = String::new();
                if rom.cs1_state != SdrrCsState::NotUsed {
                    cs.push_str(&format!("Chip Select 1: {} ", rom.cs1_state));
                }
                if rom.cs2_state != SdrrCsState::NotUsed {
                    cs.push_str(&format!("Chip Select 2: {} ", rom.cs2_state));
                }
                if rom.cs3_state != SdrrCsState::NotUsed {
                    cs.push_str(&format!("Chip Select 3: {} ", rom.cs3_state));
                }
                let rom_type = rom.rom_type;
                println!("    ROM {j}: {rom_type} {cs}");
                if verbose {
                    println!(
                        "      Flash location 0x{set_location:08x} size 0x{set_image_size:08x} bytes"
                    );
                }
                if let Some(filename) = &rom.filename {
                    println!("      Image source: {filename}");
                }
            }
        }
        Ok(())
    } else {
        Err(Error::Other(
            "No recognised information found on device flash".to_string(),
        ))
    }
}

pub async fn cmd_image(options: &Options, _args: &InspectImageArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("inspect image".into()))
}

async fn read_and_output(
    device: &Device,
    address: u32,
    length: u32,
    out: Option<&String>,
) -> Result<(), Error> {
    let data = read_memory(device, address, length).await?;

    if let Some(filename) = out {
        std::fs::write(filename, &data)?;
    } else {
        utils::print_hex_dump(address, &data);
    }

    Ok(())
}

pub async fn cmd_live(options: &Options, args: &InspectLiveArgs) -> Result<(), Error> {
    const LIVE_ROM_BASE: u32 = 0x9000_0000;
    const LIVE_ROM_MAX_OFFSET: u32 = 0x0008_0000;

    utils::check_device(options)?;
    let device = options.device.as_ref().unwrap();

    if device.state != DeviceState::Running {
        return Err(Error::NotRunning);
    }

    if args.address >= LIVE_ROM_MAX_OFFSET {
        return Err(Error::InvalidMemoryRange(args.address, args.length));
    }

    // To do: check the address range is within the correct size for this ROM
    // type

    let address = LIVE_ROM_BASE + args.address;
    read_and_output(device, address, args.length, args.out.as_ref()).await
}

pub async fn cmd_memory(options: &Options, args: &InspectMemoryArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let device = options.device.as_ref().unwrap();
    read_and_output(device, args.address, args.length, args.out.as_ref()).await
}

pub async fn cmd_gpio(options: &Options, _args: &InspectGpioArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("inspect gpio".into()))
}
