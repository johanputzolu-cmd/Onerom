// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use onerom_config::mcu::Variant;
use onerom_fw::net::Releases;

use crate::{
    args,
    utils::{check_device, check_device_nand_board, get_supported_boards},
};
use onerom_cli::{Error, Options};

pub async fn cmd_build(
    options: &Options,
    args: &args::firmware::FirmwareBuildArgs,
) -> Result<(), Error> {
    check_device_nand_board(options, &args.board)?;
    Err(Error::Unimplemented("firmware build".to_string()))
}

pub async fn cmd_inspect(
    options: &Options,
    args: &args::firmware::FirmwareInspectArgs,
) -> Result<(), Error> {
    check_device(options, args)?;
    Err(Error::Unimplemented("firmware inspect".to_string()))
}

pub async fn cmd_releases(
    options: &Options,
    args: &args::firmware::FirmwareReleasesArgs,
) -> Result<(), Error> {
    check_device_nand_board(options, &args.board)?;

    let board = if let Some(device) = options.device.as_ref() {
        let board = device
            .onerom
            .as_ref()
            .and_then(|o| o.flash.as_ref())
            .and_then(|f| f.board)
            .ok_or_else(|| {
                Error::Other("Could not determine board type from device".to_string())
            })?;
        Some(board)
    } else if let Some(board) = &args.board {
        let board = onerom_config::hw::Board::try_from_str(board)
            .ok_or_else(|| Error::InvalidBoard(board.clone(), get_supported_boards()))?;
        Some(board)
    } else {
        None
    };

    let releases = Releases::from_network()?;

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
    check_device_nand_board(options, &Some(args.board.clone()))?;

    Err(Error::Unimplemented("firmware download".to_string()))
}
