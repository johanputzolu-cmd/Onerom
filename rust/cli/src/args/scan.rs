// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Argument definitions for `onerom scan`.

use crate::args::CommandTrait;
use clap::Args;

/// Discover and list all connected One ROM devices.
///
/// Displays each device's serial number, USB location, user-assigned name
/// (if set), board type, MCU, and currently loaded firmware version.
///
/// Example:
///
///   onerom scan
///
///   onerom scan --board fire-24-e
#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Only show devices matching this board type (e.g. fire-24-e).
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,

    /// List all known board types.
    #[arg(long, conflicts_with = "board")]
    pub list_boards: bool,
}

impl CommandTrait for ScanArgs {
    fn requires_device(&self) -> bool {
        false
    }
}
