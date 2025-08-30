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

impl defmt::Format for CsActive {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            CsActive::High => defmt::write!(f, "Active High"),
            CsActive::Low => defmt::write!(f, "Active Low"),
        }
    }
}

impl CsActive {
    fn bit(&self) -> usize {
        match self {
            CsActive::High => 1,
            CsActive::Low => 0,
        }
    }
}

/// Supported types of ROMs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomType {
    // A 2364 ROM
    Type2364 {
        cs: CsActive,
    },

    // A 2332 ROM
    Type2332 {
        cs1: CsActive,
        cs2: CsActive,
    },

    // A 2316 ROM
    Type2316 {
        cs1: CsActive,
        cs2: CsActive,
        cs3: CsActive,
    },
}

impl defmt::Format for RomType {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            RomType::Type2364 { cs } => defmt::write!(f, "2364 (CS {})", cs),
            RomType::Type2332 { cs1, cs2 } => defmt::write!(f, "2332 (CS1 {}, CS2 {})", cs1, cs2),
            RomType::Type2316 { cs1, cs2, cs3 } => {
                defmt::write!(f, "2316 (CS1 {}, CS2 {}, CS3 {})", cs1, cs2, cs3)
            }
        }
    }
}

impl RomType {
    const CS_2364_ADDR: usize = 13;
    const CS1_2332_ADDR: usize = 13;
    const CS2_2332_ADDR: usize = 12;
    const CS1_2316_ADDR: usize = 13;
    const CS2_2316_ADDR: usize = 11;
    const CS3_2316_ADDR: usize = 12;

    pub fn size(&self) -> usize {
        match self {
            RomType::Type2364 { .. } => 8192,
            RomType::Type2332 { .. } => 4096,
            RomType::Type2316 { .. } => 2048,
        }
    }

    pub fn cs_mask(&self) -> usize {
        match self {
            RomType::Type2364 { cs } => cs.bit() << Self::CS_2364_ADDR,
            RomType::Type2332 { cs1, cs2 } => {
                cs1.bit() << Self::CS1_2332_ADDR | cs2.bit() << Self::CS2_2332_ADDR
            }
            RomType::Type2316 { cs1, cs2, cs3 } => {
                cs1.bit() << Self::CS1_2316_ADDR
                    | cs2.bit() << Self::CS2_2316_ADDR
                    | cs3.bit() << Self::CS3_2316_ADDR
            }
        }
    }

    pub const fn all() -> &'static [RomType] {
        &ALL_ROM_TYPES
    }
}

pub const NUM_ROM_TYPES: usize = 14;
const ALL_ROM_TYPES: [RomType; NUM_ROM_TYPES] = [
    RomType::Type2364 { cs: CsActive::Low },
    RomType::Type2364 { cs: CsActive::High },
    RomType::Type2332 {
        cs1: CsActive::Low,
        cs2: CsActive::Low,
    },
    RomType::Type2332 {
        cs1: CsActive::Low,
        cs2: CsActive::High,
    },
    RomType::Type2332 {
        cs1: CsActive::High,
        cs2: CsActive::Low,
    },
    RomType::Type2332 {
        cs1: CsActive::High,
        cs2: CsActive::High,
    },
    RomType::Type2316 {
        cs1: CsActive::Low,
        cs2: CsActive::Low,
        cs3: CsActive::Low,
    },
    RomType::Type2316 {
        cs1: CsActive::Low,
        cs2: CsActive::Low,
        cs3: CsActive::High,
    },
    RomType::Type2316 {
        cs1: CsActive::Low,
        cs2: CsActive::High,
        cs3: CsActive::Low,
    },
    RomType::Type2316 {
        cs1: CsActive::Low,
        cs2: CsActive::High,
        cs3: CsActive::High,
    },
    RomType::Type2316 {
        cs1: CsActive::High,
        cs2: CsActive::Low,
        cs3: CsActive::Low,
    },
    RomType::Type2316 {
        cs1: CsActive::High,
        cs2: CsActive::Low,
        cs3: CsActive::High,
    },
    RomType::Type2316 {
        cs1: CsActive::High,
        cs2: CsActive::High,
        cs3: CsActive::Low,
    },
    RomType::Type2316 {
        cs1: CsActive::High,
        cs2: CsActive::High,
        cs3: CsActive::High,
    },
];
