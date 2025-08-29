//! One ROM Lab - ROM Database

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use core::num::Wrapping;
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use hex_literal::hex;
use sha1::{Digest, Sha1};

/// Known ROM database
///
/// Produced either by checksumming ROMs that I own, or from zimmers.net
///
/// This script was used to generate a 32-bit wrapping checksum:
/// ```bash
/// python -c "import sys; print(f'0x{sum(open(sys.argv[1], \"rb\").read()) & 0xFFFFFFFF:08x}')"
/// ```
pub const KNOWN_ROMS: &[Entry] = &[
    Entry::new(
        "C64 BASIC",
        "901226-01",
        0x000e3d56,
        hex!("79015323128650c742a3694c9429aa91f355905e"),
    ),
    Entry::new(
        "C64 KERNAL (Rev 1)",
        "901227-01",
        0x000fd4fd,
        hex!("87cc04d61fc748b82df09856847bb5c2754a2033"),
    ),
    Entry::new(
        "C64 KERNAL (Rev 2)",
        "901227-02",
        0x000fc70b,
        hex!("0e2e4ee3f2d41f00bed72f9ab588b83e306fdb13"),
    ),
    Entry::new(
        "C64 KERNAL (Rev 3)",
        "901227-03",
        0x000fc70a,
        hex!("1d503e56df85a62fee696e7618dc5b4e781df1bb"),
    ),
    Entry::new(
        "C64 Character English",
        "901225-01",
        0x0007f7f8,
        hex!("adc7c31e18c7c7413d54802ef2f4193da14711aa"),
    ),
];

// Type agonostic wrapping checksum function
pub fn checksum<T>(data: &[u8]) -> T
where
    T: Default + From<u8> + Copy,
    Wrapping<T>: core::ops::Add<Output = Wrapping<T>>,
{
    let mut checksum = Wrapping(T::default());
    for &byte in data {
        checksum = checksum + Wrapping(T::from(byte));
    }
    checksum.0
}

pub fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut sha1 = [0u8; 20];
    sha1.copy_from_slice(&result);
    sha1
}

pub struct Entry {
    name: &'static str,
    part: &'static str,
    sum: u32,
    sha1: [u8; 20],
}

impl Entry {
    pub const fn new(name: &'static str, part: &'static str, sum: u32, sha1: [u8; 20]) -> Self {
        Self {
            name,
            part,
            sum,
            sha1,
        }
    }

    pub fn sum8(&self) -> u8 {
        (self.sum & 0xFF) as u8
    }

    pub fn sum16(&self) -> u16 {
        (self.sum & 0xFFFF) as u16
    }

    pub fn sum(&self) -> u32 {
        self.sum
    }

    pub fn matches_checksum(&self, sum: u32) -> bool {
        self.sum == sum
    }

    pub fn matches_sha1(&self, sha1: &[u8; 20]) -> bool {
        self.sha1 == *sha1
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn part(&self) -> &'static str {
        self.part
    }

    pub fn sha1(&self) -> &[u8; 20] {
        &self.sha1
    }
}

fn identify_rom_checksum(sum: u32) -> impl Iterator<Item = &'static Entry> {
    KNOWN_ROMS
        .iter()
        .filter(move |rom| rom.matches_checksum(sum))
}

fn identify_rom_sha1(sha1: &[u8; 20]) -> impl Iterator<Item = &'static Entry> {
    KNOWN_ROMS.iter().filter(move |rom| rom.matches_sha1(sha1))
}

/// Function to identify a ROM by SHA1, falling back to checksum if no SHA1 match.
pub fn identify_rom(sha1: &[u8; 20], sum: u32) -> Option<&Entry> {
    let mut roms = identify_rom_sha1(sha1);
    let (first, second) = match roms.next() {
        None => {
            let mut roms = identify_rom_checksum(sum);
            let rom = roms.next()?;
            (rom, roms.next())
        }
        Some(rom) => (rom, roms.next()),
    };

    if second.is_some() {
        error!("Multiple ROM matches for SHA1 {}:", hex::encode(sha1));
    }
    Some(first)
}
