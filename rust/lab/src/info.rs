//! One ROM Lab - Firmware Information

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use crate::types::{FlashInfo, RamInfo};

unsafe extern "C" {
    static _SEGGER_RTT: u8;
}

include!(concat!(env!("OUT_DIR"), "/built.rs"));

#[allow(static_mut_refs)]
#[allow(improper_ctypes)]
#[unsafe(link_section = ".lab_flash_info")]
#[used]
static LAB_FLASH_INFO: FlashInfo = FlashInfo {
    magic: *b"ONEL",
    major_version: PKG_VERSION_MAJOR,
    minor_version: PKG_VERSION_MINOR,
    patch_version: PKG_VERSION_PATCH,
    build_number: "",
    mcu: "f405rg",
    hw_rev: "24-f",
    rtt: unsafe { &_SEGGER_RTT as *const u8 as *const core::ffi::c_void },
    reserved: [0; 200],
};

#[unsafe(link_section = ".lab_ram_info")]
#[used]
pub static mut LAB_RAM_INFO: RamInfo = RamInfo {
    magic: *b"onel",
    rom_data: core::ptr::null(),
    reserved: [0; 248],
};

