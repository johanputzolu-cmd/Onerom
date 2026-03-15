// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use log::{debug, trace};
use std::io::Write;

use onerom_config::fw::{FirmwareProperties, FirmwareVersion, ServeAlg};
use onerom_config::hw::Board;
use onerom_config::mcu::Variant;
use onerom_fw::net::{Release, Releases, fetch_license_async};
use onerom_fw::{assemble_firmware, get_rom_files_async, read_rom_config, validate_sizes};
use onerom_gen::{Builder, FIRMWARE_SIZE, License};
use sdrr_fw_parser::{Parser, SdrrInfo, readers::MemoryReader};

use crate::args;
use crate::utils::{resolve_board, resolve_firmware_output};
use onerom_cli::{Error, Options};

pub const EMPTY_CONFIG_FILE: &str = r#"{
    "$schema": "https://images.onerom.org/configs/schema.json",
    "version": 1,
    "name": "Empty config",
    "description": "Created by the One ROM CLI",
    "rom_sets": []
}"#;

pub async fn verify_assembled_firmware(
    options: &Options,
    data: &[u8],
    force: bool,
) -> Result<(), Error> {
    let info = parse_firmware(data).await?;
    if !info.parse_errors.is_empty() {
        let detail = info
            .parse_errors
            .iter()
            .map(|e| format!("  {e}"))
            .collect::<Vec<_>>()
            .join("\n");
        if force {
            eprintln!("Warning: assembled firmware has parse errors (continuing due to --force):");
            eprintln!("{detail}");
        } else {
            return Err(Error::FirmwareValidation(detail));
        }
    } else if options.verbose {
        println!(
            "Assembled firmware version {} parsed successfully with no errors",
            info.version
        );
    }
    Ok(())
}

pub async fn parse_firmware(data: &[u8]) -> Result<SdrrInfo, Error> {
    // The hardcoded base address looks odd here, as the STM32's base flash
    // address, but when using a memory reader, sdrr-fw-parse will just figure
    // it out for itself based on what it finds in the image.
    let mut reader = MemoryReader::new(data.to_vec(), 0x0800_0000);
    let mut parser = Parser::new(&mut reader);
    parser.parse_flash().await.map_err(Error::Other)
}

fn check_firmware_size(options: &Options, data: &[u8]) -> Result<(), Error> {
    if options.verbose {
        println!("Firmware size {} bytes", data.len());
    }
    if data.len() > FIRMWARE_SIZE {
        return Err(Error::FirmwareTooLarge(data.len(), FIRMWARE_SIZE));
    }
    Ok(())
}

fn resolve_release<'a>(
    releases: &'a Releases,
    version: &Option<String>,
) -> Result<&'a Release, Error> {
    if let Some(version) = version {
        releases
            .release_from_string(version)
            .ok_or_else(|| Error::VersionNotFound(version.clone(), releases.releases_str()))
    } else {
        releases
            .release_from_string(releases.latest())
            .ok_or(Error::NoLatestRelease)
    }
}

pub async fn acquire_firmware(
    options: &Options,
    firmware_path: &Option<String>,
    version_arg: &Option<String>,
    board: &Board,
    mcu: &Variant,
) -> Result<(Vec<u8>, FirmwareVersion, String), Error> {
    if let Some(firmware) = firmware_path {
        if options.verbose {
            println!("Using local firmware: {firmware}");
        }
        let data = std::fs::read(firmware)?;
        check_firmware_size(options, &data)?;

        let info = parse_firmware(&data).await?;
        let version_str = format!("{}", info.version);
        if options.verbose {
            println!("Detected firmware version: {version_str}");
        }
        Ok((data, info.version, version_str))
    } else {
        if options.verbose {
            println!("Checking available firmware versions...");
        }
        let releases = Releases::from_network_async().await?;
        let release = resolve_release(&releases, version_arg)?;
        let version = release.firmware_version()?;
        let version_str = release.version.clone();

        if options.verbose {
            println!(
                "Downloading firmware v{version_str} for {}...",
                board.name()
            );
        }
        let data = releases
            .download_firmware_async(&version, board, mcu)
            .await?;
        check_firmware_size(options, &data)?;
        Ok((data, version, version_str))
    }
}

pub async fn build_rom_image(
    options: &Options,
    config: Option<&str>,
    version: FirmwareVersion,
    board: Board,
    mcu: Variant,
) -> Result<(FirmwareProperties, Option<Vec<u8>>, Option<Vec<u8>>, String), Error> {
    let rom_config = if let Some(config) = config {
        read_rom_config(config)?
    } else {
        EMPTY_CONFIG_FILE.to_string()
    };
    let mut builder =
        Builder::from_json(version, mcu.family(), &rom_config).map_err(onerom_fw::Error::parse)?;

    for license in builder.licenses() {
        accept_license(options, &license).await?;
        builder
            .accept_license(&license)
            .map_err(onerom_fw::Error::license)?;
    }

    get_rom_files_async(&mut builder).await?;

    let fw_props = FirmwareProperties::new(version, board, mcu, ServeAlg::default(), true)?;
    let (metadata, image_data) = builder.build(fw_props).map_err(onerom_fw::Error::build)?;

    let metadata = if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    };
    let image_data = if image_data.is_empty() {
        None
    } else {
        Some(image_data)
    };
    let desc = builder.description();

    Ok((fw_props, metadata, image_data, desc))
}

fn check_build_args(
    _options: &Options,
    args: &args::firmware::FirmwareBuildArgs,
) -> Result<(), Error> {
    if !args.no_config && args.config_file.is_none() && args.rom.is_empty() {
        return Err(Error::InvalidArgument(
            "Either --config or --rom must be specified unless --no-config is set".to_string(),
        ));
    }
    if args.no_config && (!args.rom.is_empty() || args.config_file.is_some()) {
        return Err(Error::InvalidArgument(
            "--no-config cannot be used with --rom or --config".to_string(),
        ));
    }
    Ok(())
}

pub async fn cmd_build(
    options: &Options,
    args: &args::firmware::FirmwareBuildArgs,
) -> Result<(), Error> {
    // Build args are too complicated to fully handle via clap
    check_build_args(options, args)?;

    let board = resolve_board(options, &args.board)?.ok_or(Error::NoBoardOrDevice)?;

    // Hardcode MCU as CLI only supports Fire boards
    let mcu = Variant::RP2350;

    let config = if args.no_config {
        if options.verbose {
            println!("No config file specified, proceeding without ROM images");
        }
        None
    } else if let Some(config) = args.config_file.as_ref() {
        if options.verbose {
            println!("Using ROM config: {config}");
        }
        Some(config.as_str())
    } else {
        return Err(Error::InvalidArgument(
            "--rom not currently supported".to_string(),
        ));
    };

    let (firmware_data, version, version_str) =
        acquire_firmware(options, &args.base_firmware, &args.version, &board, &mcu).await?;

    let (fw_props, metadata, image_data, desc) =
        build_rom_image(options, config, version, board, mcu).await?;

    validate_sizes(&fw_props, &firmware_data, &metadata, &image_data)?;

    let assembled = assemble_firmware(firmware_data, metadata, image_data)?;
    let size = assembled.len();
    verify_assembled_firmware(options, &assembled, args.force).await?;

    let out = resolve_firmware_output(
        &args.output,
        &args.path,
        &board,
        Some(&version_str),
        args.config_file.as_deref(),
    );
    std::fs::write(&out, &assembled)?;

    if options.verbose {
        println!("Wrote {} bytes to {}", size, out);
        if !desc.is_empty() {
            println!("---\n{desc}");
        }
    } else {
        println!("Firmware written to {}", out);
    }

    Ok(())
}

pub async fn accept_license(options: &Options, license: &License) -> Result<(), Error> {
    let text = fetch_license_async(&license.url).await?;

    println!("License required:");
    println!("---");
    println!("{text}");
    println!("---");

    if options.yes {
        println!("Auto-accepted (--yes)");
        return Ok(());
    }

    print!("Do you accept this license? (y/N): ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err(Error::LicenseNotAccepted),
    }
}

pub async fn cmd_inspect(
    options: &Options,
    args: &args::firmware::FirmwareInspectArgs,
) -> Result<(), Error> {
    let data = if let Some(file) = &args.firmware {
        if options.verbose {
            println!("Inspecting local firmware: {file}");
        }
        std::fs::read(file)?
    } else {
        let board = resolve_board(options, &args.board)?.ok_or(Error::NoBoardOrDevice)?;
        let mcu = Variant::RP2350;

        let releases = Releases::from_network_async().await?;
        let release = resolve_release(&releases, &args.version)?;
        let version = release.firmware_version()?;

        if options.verbose {
            println!(
                "Downloading firmware v{} for {}...",
                release.version,
                board.name()
            );
        }
        releases
            .download_firmware_async(&version, &board, &mcu)
            .await?
    };
    if options.verbose {
        println!("Firmware size: {} bytes", data.len());
    }

    let info = parse_firmware(&data).await?;
    if !info.parse_errors.is_empty() {
        eprintln!("Warning: firmware parsed with errors:");
        for error in &info.parse_errors {
            eprintln!("  {error}");
        }
        eprintln!();
    }

    if !options.verbose {
        println!("Version:  {}", info.version);
        if let Some(hw_rev) = &info.hw_rev {
            println!("Hardware: {hw_rev}");
        }
        println!("MCU:      {:?}", info.stm_line);
        println!("ROM sets: {}", info.rom_set_count);

        for (i, set) in info.rom_sets.iter().enumerate() {
            println!("  Set {i}: {} ROM(s), {} bytes", set.rom_count, set.size);
            for (j, rom) in set.roms.iter().enumerate() {
                let name = rom.filename.as_deref().unwrap_or("<unnamed>");
                println!("    ROM {j}: {} {name}", rom.rom_type);
            }
        }
    } else {
        let json = serde_json::to_string_pretty(&info).map_err(|e| Error::Other(e.to_string()))?;
        println!("---");
        println!("{json}");
    }

    Ok(())
}

pub async fn cmd_releases(
    options: &Options,
    args: &args::firmware::FirmwareReleasesArgs,
) -> Result<(), Error> {
    let board = if args.all {
        trace!("Showing all releases (including those for attached device if present)");
        None
    } else {
        trace!("Resolving board to filter releases");
        resolve_board(options, &args.board)?
    };
    debug!("Resolved board for releases: {board:?}");

    let releases = Releases::from_network_async().await?;

    let filtered = if let Some(board) = &board {
        releases
            .releases()
            .iter()
            .filter(|r| {
                r.boards
                    .iter()
                    .any(|b| b.name == board.name().to_ascii_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>()
    } else {
        releases.releases().clone()
    };

    if filtered.is_empty() {
        println!("No releases found.");
        return Ok(());
    }

    if let Some(board) = &board {
        println!("Available firmware releases for {board}:");
    } else {
        println!("Available firmware releases:");
    }
    for r in &filtered {
        let latest = if r.version == releases.latest() {
            " (latest)"
        } else {
            ""
        };

        println!("  v{}{latest}", r.version);
        if options.verbose {
            let boards = r
                .boards
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(board) = board.as_ref() {
                let url = r.url(board, &Variant::RP2350)?;
                println!("    Location: {url}");
            }
            println!("    Supported boards: {boards}");
        }
    }

    Ok(())
}

pub async fn cmd_download(
    options: &Options,
    args: &args::firmware::FirmwareDownloadArgs,
) -> Result<(), Error> {
    let board = resolve_board(options, &args.board)?.ok_or(Error::NoBoardOrDevice)?;
    let mcu = Variant::RP2350;

    let releases = Releases::from_network_async().await?;
    let release = resolve_release(&releases, &args.version)?;
    let version = release.firmware_version()?;

    if options.verbose {
        println!(
            "Downloading firmware v{} for {}...",
            release.version,
            board.name()
        );
    }
    let data = releases
        .download_firmware_async(&version, &board, &mcu)
        .await?;
    check_firmware_size(options, &data)?;

    let out = resolve_firmware_output(
        &args.output,
        &args.path,
        &board,
        Some(&release.version),
        None,
    );
    std::fs::write(&out, &data)?;

    if options.verbose {
        println!("Written {} bytes to {}", data.len(), out);
    } else {
        println!("Firmware downloaded to {}", out);
    }

    Ok(())
}
