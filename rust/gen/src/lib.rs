// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Generates firmware artifacts for One ROM.

#![no_std]

extern crate alloc;

pub mod builder;
pub mod image;
pub mod meta;

use alloc::string::String;

use onerom_config::fw::ServeAlg;

use crate::image::CsLogic;

/// Error type
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
    },
    MissingPointer {
        id: usize,
    },
    InvalidServeAlg {
        serve_alg: ServeAlg,
    },
    InconsistentCsLogic {
        first: CsLogic,
        other: CsLogic,
    },
    InvalidConfig {
        error: String,
    },
    UnsupportedConfigVersion {
        version: u32,
    },
    DuplicateFile {
        id: usize,
    },
    InvalidFile {
        id: usize,
        total: usize,
    },
    MissingFile {
        id: usize,
    },
}
type Result<T> = core::result::Result<T, Error>;
