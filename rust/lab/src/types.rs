//! One ROM Lab - Types

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

/// Whether a CS line is active low or active high
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CsActive {
    #[allow(dead_code)]
    High,

    #[default]
    Low,
}

/// Supported types of ROMs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomType {
    // A 2364 ROM
    Type2364{ cs: CsActive },

    // A 2332 ROM
    Type2332{ cs1: CsActive, cs2: CsActive },

    // A 2316 ROM
    Type2316{ cs1: CsActive, cs2: CsActive, cs3: CsActive },
}