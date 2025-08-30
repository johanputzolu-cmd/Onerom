//! One ROM Lab - ROM Database
//!
//! The primary approach taken to identifying ROMs is to use a SHA1 digest of
//! it.  If that fails to provide a match, we try a 32-bit summing checksum.
//!
//! This combination will always, uniquely, identify a ROM - in fact the SHA1
//! digest should.  This has the side effect of a unique ROM image needing a
//! single name and part number in the database.  If this turns out to be an
//! unsound assumption, we may need to add the concept of aliases.

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use alloc::vec::Vec;
use core::num::Wrapping;
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use hex_literal::hex;
use sha1::{Digest, Sha1};

use crate::{CsActive, RomType};

// Known ROM database
//
// This script was used to generate a 32-bit wrapping checksum:
// ```bash
// python -c "import sys; print(f'0x{sum(open(sys.argv[1], \"rb\").read()) & 0xFFFFFFFF:08x}')" filename
// ```
include!(concat!(env!("OUT_DIR"), "/roms.rs"));

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

#[derive(Debug)]
pub struct Entry {
    name: &'static str,
    part: &'static str,
    sum: u32,
    sha1: [u8; 20],
    rom_type: RomType,
}

impl Entry {
    pub const fn new(
        name: &'static str,
        part: &'static str,
        sum: u32,
        sha1: [u8; 20],
        rom_type: RomType,
    ) -> Self {
        Self {
            name,
            part,
            sum,
            sha1,
            rom_type,
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

    pub fn rom_type(&self) -> RomType {
        self.rom_type
    }
}

fn identify_rom_checksum(sum: u32) -> impl Iterator<Item = &'static Entry> {
    ROMS.iter().filter(move |rom| rom.matches_checksum(sum))
}

fn identify_rom_sha1(sha1: &[u8; 20]) -> impl Iterator<Item = &'static Entry> {
    ROMS.iter().filter(move |rom| rom.matches_sha1(sha1))
}

/// Function to identify a ROM by SHA1, falling back to checksum if no SHA1
/// match.
pub fn identify_rom(
    rom_type: &RomType,
    sum: u32,
    sha1: [u8; 20],
) -> (Vec<&'static Entry>, Vec<(&'static Entry, RomType)>) {
    let mut candidates = identify_rom_sha1(&sha1).collect::<Vec<_>>();

    if candidates.is_empty() {
        candidates = identify_rom_checksum(sum).collect::<Vec<_>>();
    }

    let mut matches = Vec::new();
    let mut wrong_type_matches = Vec::new();

    for entry in candidates {
        if entry.rom_type == *rom_type {
            matches.push(entry);
        } else {
            wrong_type_matches.push((entry, entry.rom_type));
        }
    }

    (matches, wrong_type_matches)
}
