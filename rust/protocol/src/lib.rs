//! One ROM Protocol

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#![no_std]

extern crate alloc;

pub mod lab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Buffer too small for the data
    BufferTooSmall,
    /// Response was not as expected
    InvalidResponse,
    /// Invalid data received
    InvalidData,
}