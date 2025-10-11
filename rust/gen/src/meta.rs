// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Metadata generator for One ROM.
//!
//! 

use onerom_config::McuFamily;

pub struct Header {
    magic: [u8; 16],
    version: u32,
    rom_set_count: u8,
    pad1: [u8; 3],
    rom_sets: u32,
    reserved: [u8; 228],
}

impl Header {
    pub fn new(family: McuFamily, rom_set_count: u8) -> Self {
        Self {
            magic: *b"ONEROM_METADATA\0",
            version: 1,
            rom_set_count,
            pad1: [0; 3],
            rom_sets: 0,
            reserved: [0; 228],
        }
    }
}