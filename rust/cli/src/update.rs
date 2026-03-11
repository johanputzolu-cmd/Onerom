// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use crate::{args, utils};
use onerom_cli::{Error, Options};

pub async fn cmd_flash(
    options: &Options,
    _args: &args::update::UpdateFlashArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("update flash".to_string()))
}

pub async fn cmd_commit(
    options: &Options,
    _args: &args::update::UpdateCommitArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("update commit".to_string()))
}

pub async fn cmd_rename(
    options: &Options,
    _args: &args::update::UpdateRenameArgs,
) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("update rename".to_string()))
}

pub async fn cmd_otp(options: &Options, _args: &args::update::UpdateOtpArgs) -> Result<(), Error> {
    utils::check_device(options)?;
    let _device = options.device.as_ref().unwrap();
    Err(Error::Unimplemented("update otp".to_string()))
}
