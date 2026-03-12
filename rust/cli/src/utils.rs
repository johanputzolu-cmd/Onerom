// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use std::io::Write;

use crate::args::CommandTrait;
use onerom_cli::{DeviceState, Error, LogLevel, Options};

pub fn get_supported_boards() -> String {
    onerom_config::hw::BOARDS
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn init_logging(options: &Options) {
    let log_level = &options.log_level;

    let mut log_builder = env_logger::Builder::from_default_env();

    match log_level {
        LogLevel::Warn => {
            log_builder.filter_level(log::LevelFilter::Warn);
        }
        LogLevel::Info => {
            log_builder.filter_level(log::LevelFilter::Info);
            // nusb is noisy at info level
            log_builder.filter_module("nusb", log::LevelFilter::Warn);
        }
        LogLevel::Debug => {
            log_builder.filter_level(log::LevelFilter::Debug);
            // nusb is very noisy at debug level
            log_builder.filter_module("nusb", log::LevelFilter::Info);
        }
        LogLevel::Trace => {
            log_builder.filter_level(log::LevelFilter::Trace);
        }
    }

    log_builder.format(|buf, record| {
        let level = format!("{}: ", record.level());
        writeln!(buf, "{:07}{}", level, record.args())
    });
    log_builder.init();
}

pub fn check_device_nand_board(options: &Options, board_arg: &Option<String>) -> Result<(), Error> {
    if options.device.is_some() && board_arg.is_some() {
        return Err(Error::DeviceAndBoard);
    }
    Ok(())
}

/// Checks that a device is required and present if the command needs one.
pub fn check_device(options: &Options, args: &impl CommandTrait) -> Result<(), Error> {
    if args.requires_device() && options.device.is_none() {
        return Err(Error::NoDevice);
    }
    Ok(())
}

pub fn parse_u32(s: &str) -> Result<u32, std::num::ParseIntError> {
    let s = s.replace('_', "");
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        s.parse::<u32>()
    }
}

pub fn parse_u8(s: &str) -> Result<u8, std::num::ParseIntError> {
    let s = s.replace('_', "");
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16)
    } else {
        s.parse::<u8>()
    }
}

pub fn print_hex_dump(address: u32, data: &[u8]) {
    const BYTES_PER_ROW: usize = 16;
    const GROUP_SIZE: usize = 4;

    for (row_idx, row) in data.chunks(BYTES_PER_ROW).enumerate() {
        let row_addr = address + (row_idx * BYTES_PER_ROW) as u32;

        // Address
        print!("0x{:08x}  ", row_addr);

        // Hex bytes in groups of 4
        for (i, chunk) in row.chunks(GROUP_SIZE).enumerate() {
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            // Pad if this chunk was short (last row)
            if chunk.len() < GROUP_SIZE {
                let missing = GROUP_SIZE - chunk.len();
                print!("{}", "   ".repeat(missing));
            }
            if i < (BYTES_PER_ROW / GROUP_SIZE) - 1 {
                print!(" ");
            }
        }

        // Pad if the whole row was short
        if row.len() < BYTES_PER_ROW {
            let missing_bytes = BYTES_PER_ROW - row.len();
            let missing_groups = missing_bytes / GROUP_SIZE;
            let _ = missing_groups; // already padded per-chunk above
        }

        // ASCII
        print!(" |");
        for byte in row {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            print!("{}", ch);
        }
        println!("|");
    }
}

/// Checks an address offset and length for validity against this particular
/// device.
///
/// Checks the device is running and can accept live reads/writes.
/// Checks that the offset is valid for the ROM currently being served by
/// the devce.
///
/// Returns the actual device start address to read/write.
pub fn check_live_read_write(
    options: &Options,
    offset: u32,
    length: u32,
    args: &impl CommandTrait,
) -> Result<u32, Error> {
    const LIVE_ROM_BASE: u32 = 0x9000_0000;
    const LIVE_ROM_MAX_OFFSET: u32 = 0x0008_0000;

    check_device(options, args)?;
    let device = options.device.as_ref().unwrap();

    if device.state != DeviceState::Running {
        return Err(Error::NotRunning);
    }

    let end_offset = offset + length;
    if end_offset >= LIVE_ROM_MAX_OFFSET {
        return Err(Error::InvalidMemoryRange(offset, length));
    }

    // To do: check the address range is within the correct size for this ROM
    // type

    Ok(LIVE_ROM_BASE + offset)
}
