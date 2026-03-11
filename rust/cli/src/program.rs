// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::{args, utils};
use onerom_cli::{Error, Options};

pub async fn cmd_program(
    options: &Options,
    args: &args::program::ProgramArgs,
) -> Result<(), Error> {
    utils::check_device_nand_board(options, &args.board)?;
    Err(Error::Unimplemented("program".to_string()))
}
