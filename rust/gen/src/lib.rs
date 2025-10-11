// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Generates firmware artifacts for One ROM.

#![no_std]

extern crate alloc;

pub mod rom;

/// Error type
#[derive(Debug)]
pub enum Error {
    RightSize {
        size: usize,
    },
    RomTooSmall {
        expected: usize,
        actual: usize,
    },
    DuplicationNotExactDivisor {
        rom_size: usize,
        expected_size: usize,
    },
    RomTooLarge {
        rom_size: usize,
        expected_size: usize,
    },
    BufferTooSmall {
        expected: usize,
        actual: usize,
    },
    NoRoms,
    TooManyRoms {
        expected: usize,
        actual: usize,
    },
    MissingCsConfig {
        line: &'static str,        
    }
}
type Result<T> = core::result::Result<T, Error>;
