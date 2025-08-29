//! One ROM Lab - Checksum

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use core::num::Wrapping;

// Known ROM database
pub const KNOWN_ROMS: &[RomChecksum] = &[
    RomChecksum::new("C64 BASIC", "901226-01", 0x000e3d56),
    RomChecksum::new("C64 KERNAL", "901227-03", 0x000fc70a),
    RomChecksum::new("C64 Character", "901225-01", 0x0007f7f8),
];

// Type agonostic wrapping checksum function
pub fn checksum<T>(data: &[u8]) -> T 
where 
    T: Default + From<u8> + Copy,
    Wrapping<T>: core::ops::Add<Output = Wrapping<T>>
{
    let mut checksum = Wrapping(T::default());
    for &byte in data {
        checksum = checksum + Wrapping(T::from(byte));
    }
    checksum.0
}

pub struct RomChecksum {
    name: &'static str,
    part: &'static str,
    sum32: u32,
}

impl RomChecksum {
    pub const fn new(name: &'static str, part: &'static str, sum32: u32) -> Self {
        Self { name, part, sum32 }
    }

    pub fn sum8(&self) -> u8 {
        (self.sum32 & 0xFF) as u8
    }

    pub fn sum16(&self) -> u16 {
        (self.sum32 & 0xFFFF) as u16
    }

    pub fn sum32(&self) -> u32 {
        self.sum32
    }

    pub fn matches(&self, sum32: u32) -> bool {
        self.sum32 == sum32
    }
    
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn part(&self) -> &'static str {
        self.part
    }
}

pub fn identify_rom(sum32: u32) -> impl Iterator<Item = &'static RomChecksum> {
    KNOWN_ROMS.iter().filter(move |rom| rom.matches(sum32))
}