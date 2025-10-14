// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! onerom-fw - Firmware generator for One ROM

mod args;
mod error;
mod net;

use clap::Parser;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::io::Write;

use onerom_config::fw::{FirmwareProperties, ServeAlg};
use onerom_gen::builder::{Builder, FileData, License};
use onerom_gen::{FIRMWARE_SIZE, MAX_METADATA_LEN};

use args::Args;
pub use error::Error;
use net::{fetch_license, fetch_rom_file, Releases};

fn main() -> Result<(), Error> {
    // Get args
    let mut args = Args::parse();
    if args.validate()? {
        return Ok(())
    }

    // Enable logging
    init_logging(args.verbose);

    // Output version
    debug!("onerom-fw version {}", env!("CARGO_PKG_VERSION"));

    // Get firmware releases
    let releases = Releases::from_network()?;
    let version = args.fw_version.unwrap();
    if releases.release(&version).is_none() {
        return Err(Error::release_not_found());
    }

    // Get the blank firmware image
    let board = args.board.unwrap();
    let mcu = args.mcu.unwrap();
    let firmware_data = releases.download_firmware(&version, &board, &mcu)?;

    // Build firmware properties
    let fw_props = FirmwareProperties::new(
        version,
        board,
        mcu,
        ServeAlg::default(),
        true,
    ).unwrap();

    // Generate metadata/ROM images
    let (metadata, image_data) = if let Some(rom_config) = &args.rom {
        let (m, i) = process_rom_config(fw_props, rom_config)?;
        if i.len() > 0 {
            // Cannot have ROM image data without metadata
            assert!(m.len() > 0);
        }
        (Some(m), Some(i))
    } else {
        println!("No ROM config specified, creating firmware with no metadata or image data");
        (None, None)
    };

    // Check everything fits
    validate_sizes(&fw_props, &firmware_data, &metadata, &image_data)?;

    // Create the firmware file
    let filename = args.out.as_ref().unwrap();
    let size = create_firmware(
        &filename,
        firmware_data,
        metadata,
        image_data,
    )?;

    println!("Successfully created firmware file `{}` ({} bytes)", filename, size);

    // Done
    Ok(())
}

fn validate_sizes(fw_props: &FirmwareProperties, firmware_data: &[u8], metadata: &Option<Vec<u8>>, image_data: &Option<Vec<u8>>) -> Result<(), Error> {
    let mut total_size = 0;

    let fw_size = firmware_data.len();
    debug!("Firmware size: {} bytes", fw_size);
    if fw_size > FIRMWARE_SIZE {
        return Err(Error::too_large("Firmware".to_string(), fw_size, FIRMWARE_SIZE));
    }
    total_size += fw_size;

    if let Some(meta) = metadata {
        // Padding after firmware
        total_size += FIRMWARE_SIZE - total_size;

        let meta_size = meta.len();
        debug!("Metadata size: {} bytes", meta_size);
        if meta_size > MAX_METADATA_LEN {
            return Err(Error::too_large("Metadata".to_string(), meta_size, MAX_METADATA_LEN));
        }
        total_size += meta_size;
    }

    if let Some(image) = image_data {
        // Padding after metadata
        total_size += MAX_METADATA_LEN + FIRMWARE_SIZE - total_size;

        let image_size = image.len();
        debug!("Image data size: {} bytes", image_size);
        total_size += image_size;
    }

    let max_size = fw_props.mcu_variant().flash_storage_bytes();
    debug!("Total firmware size: {} bytes (max {})", total_size, max_size);
    debug!("MCU flash size: {} bytes", max_size);
    if total_size > max_size {
        return Err(Error::too_large("Total firmware".to_string(), total_size, max_size));
    }

    Ok(())
}

fn create_firmware(
    out_path: &str,
    firmware_data: Vec<u8>,
    metadata: Option<Vec<u8>>,
    image_data: Option<Vec<u8>>,
) -> Result<usize, Error> {
    let mut total_size = 0;

    // Open file
    let mut out_file = std::fs::File::create(out_path).map_err(Error::write)?;

    // Output firmware data
    let firmware_size = firmware_data.len();
    assert!(firmware_size <= FIRMWARE_SIZE);
    let pad_size = FIRMWARE_SIZE - firmware_size;
    out_file.write_all(&firmware_data).map_err(Error::write)?;
    total_size += firmware_size;
    debug!("Wrote {} bytes of firmware", firmware_size);

    if metadata.is_none() {
        // Cannot have image data without metadata
        assert!(image_data.is_none());
        return Ok(total_size);
    }

    // Pad to beginning of metadata
    out_file.write_all(&vec![0xFF; pad_size]).map_err(Error::write)?;
    total_size += pad_size;
    debug!("Wrote {} bytes of padding after firmware", pad_size);

    // Write metadata
    assert!(total_size == FIRMWARE_SIZE);
    let metadata = metadata.unwrap();
    let metadata_size = metadata.len();
    assert!(metadata_size <= MAX_METADATA_LEN);
    out_file.write_all(&metadata).map_err(Error::write)?;
    total_size += metadata_size;
    debug!("Wrote {} bytes of metadata", metadata_size);

    if image_data.is_none() {
        return Ok(total_size);
    }

    // Pad to beginning of image data
    let pad_size = MAX_METADATA_LEN - metadata_size;
    debug!("Adding {} bytes of padding before image data", pad_size);
    out_file.write_all(&vec![0xFF; pad_size]).map_err(Error::write)?;
    total_size += pad_size;
    debug!("Wrote {} bytes of padding after metadata", pad_size);

    // Write image data
    assert!(total_size == FIRMWARE_SIZE + MAX_METADATA_LEN);
    let image_data = image_data.unwrap();
    let image_size = image_data.len();
    out_file.write_all(&image_data).map_err(Error::write)?;
    total_size += image_size;
    debug!("Wrote {} bytes of image data", image_size);

    Ok(total_size)
}

fn process_rom_config(fw_props: FirmwareProperties, rom_config: &str) -> Result<(Vec<u8>, Vec<u8>), Error> {
    // Load the config file
    let config = std::fs::read_to_string(rom_config).map_err(Error::read)?;

    // Create builder
    let mut builder = Builder::from_json(&config).map_err(Error::parse)?;

    // Accept any licenses
    let licenses = builder.licenses();
    for license in licenses {
        propose_license(&license)?;
        builder.accept_license(&license).map_err(Error::license)?;
    }

    // Get firmware files
    let file_specs = builder.file_specs();
    for spec in file_specs {
        let source = spec.source;
        let extract = spec.extract;
        let data = fetch_rom_file(&source, extract)?;
        
        builder.add_file(FileData {
            id: spec.id,
            data,
        }).map_err(Error::build)?;
    }

    // Build metadata and image data
    let (metadata, image_data) = builder.build(fw_props).map_err(Error::build)?;

    Ok((metadata, image_data))
}

fn propose_license(license: &License) -> Result<(), Error> {
    // Get license text
    debug!("License required: {}", license.url);
    let text = fetch_license(&license.url)?;

    // Output it
    println!("You must accept this license to proceed:");
    println!("---");
    println!("{}", text);
    println!("---");

    // Prompt user
    print!("Do you accept this license? (y/N): ");
    std::io::stdout().flush().map_err(Error::write)?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(Error::read)?;
    let input = input.trim().to_lowercase();
    if input == "y" || input == "yes" {
        Ok(())
    } else {
        Err(Error::license_not_accepted())
    }
}


fn init_logging(verbose: bool) {
    let mut log_builder = env_logger::Builder::from_default_env();
    if verbose {
        log_builder.filter_level(log::LevelFilter::Debug);
    } else {
        log_builder.filter_level(log::LevelFilter::Info);
    }
    log_builder.format(|buf, record| {
        let level = format!("{}: ", record.level());
        writeln!(buf, "{:07}{}", level, record.args())
    });
    log_builder.init();
}