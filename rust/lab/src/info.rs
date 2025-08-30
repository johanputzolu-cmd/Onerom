//! One ROM Lab - Firmware Information

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use crate::types::{FlashInfo, RamInfo};

include!(concat!(env!("OUT_DIR"), "/built.rs"));

#[unsafe(link_section = ".lab_flash_info")]
#[used]
static LAB_FLASH_INFO: FlashInfo = FlashInfo {
    magic: *b"SDRL",
    major_version: PKG_VERSION_MAJOR,
    minor_version: PKG_VERSION_MINOR,
    patch_version: PKG_VERSION_PATCH,
    build_number: "",
    mcu: "f405rg",
    hw_rev: "24-f",
    reserved: [0; 204],
};

#[unsafe(link_section = ".lab_ram_info")]
#[used]
static mut LAB_RAM_INFO: RamInfo = RamInfo {
    magic: *b"SDRM",
    reserved: [0; 252],
};
