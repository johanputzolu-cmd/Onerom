//! One ROM Lab - Types

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#[allow(unused_imports)]

/// Whether a CS line is active low or active high
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CsActive {
    #[allow(dead_code)]
    High,

    #[default]
    Low,
}

impl core::fmt::Display for CsActive {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CsActive::High => write!(f, "Active High"),
            CsActive::Low => write!(f, "Active Low"),
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

/// Supported types of ROMs.  This type includes the chip select behaviour of
/// the ROM, which was mask programmed at factory for the original ROM chips.
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

impl core::fmt::Display for RomType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({})", self.type_str(), self.cs_str())
    }
}

impl RomType {
    const CS_2364_ADDR: usize = 13;
    const CS1_2332_ADDR: usize = 13;
    const CS2_2332_ADDR: usize = 12;
    const CS1_2316_ADDR: usize = 13;
    const CS2_2316_ADDR: usize = 11;
    const CS3_2316_ADDR: usize = 12;

    /// Returns the max size of ROM supported by this object.
    pub const fn max_size() -> usize {
        8192
    }

    /// Returns the size of this ROM.
    pub const fn size(&self) -> usize {
        match self {
            RomType::Type2364 { .. } => 8192,
            RomType::Type2332 { .. } => 4096,
            RomType::Type2316 { .. } => 2048,
        }
    }

    /// Returns the active CS mask for this ROM type.
    pub fn cs_active_mask(&self) -> usize {
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

    /// Returns all supported ROM types.
    pub const fn all() -> &'static [RomType] {
        &ALL_ROM_TYPES
    }

    pub const fn type_str(&self) -> &'static str {
        match self {
            RomType::Type2364 { .. } => "2364",
            RomType::Type2332 { .. } => "2332",
            RomType::Type2316 { .. } => "2316",
        }
    }

    pub const fn cs_str(&self) -> &'static str {
        match self {
            RomType::Type2364 { cs } => match cs {
                CsActive::Low => "CS Low",
                CsActive::High => "CS High",
            },
            RomType::Type2332 { cs1, cs2 } => match (cs1, cs2) {
                (CsActive::Low, CsActive::Low) => "CS1 Low, CS2 Low",
                (CsActive::Low, CsActive::High) => "CS1 Low, CS2 High",
                (CsActive::High, CsActive::Low) => "CS1 High, CS2 Low",
                (CsActive::High, CsActive::High) => "CS1 High, CS2 High",
            },
            RomType::Type2316 { cs1, cs2, cs3 } => match (cs1, cs2, cs3) {
                (CsActive::Low, CsActive::Low, CsActive::Low) => "CS1 Low, CS2 Low, CS3 Low",
                (CsActive::Low, CsActive::Low, CsActive::High) => "CS1 Low, CS2 Low, CS3 High",
                (CsActive::Low, CsActive::High, CsActive::Low) => "CS1 Low, CS2 High, CS3 Low",
                (CsActive::Low, CsActive::High, CsActive::High) => "CS1 Low, CS2 High, CS3 High",
                (CsActive::High, CsActive::Low, CsActive::Low) => "CS1 High, CS2 Low, CS3 Low",
                (CsActive::High, CsActive::Low, CsActive::High) => "CS1 High, CS2 Low, CS3 High",
                (CsActive::High, CsActive::High, CsActive::Low) => "CS1 High, CS2 High, CS3 Low",
                (CsActive::High, CsActive::High, CsActive::High) => "CS1 High, CS2 High, CS3 High",
            },
        }
    }
}

// Enumeration of all possible ROM types.
const NUM_ROM_TYPES: usize = 14;
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

/// Information about One ROM Lab's firmware.
///
/// Note &'static str is a "fat" pointer, with 4 bytes pointer and 4 bytes
/// length.
#[repr(C)]
pub struct FlashInfo {
    pub magic: [u8; 4],
    pub major_version: &'static str,
    pub minor_version: &'static str,
    pub patch_version: &'static str,
    pub build_number: &'static str,
    pub mcu: &'static str,
    pub hw_rev: &'static str,
    pub features: &'static str,
    pub rtt: *const core::ffi::c_void,
    pub reserved: [u8; 192],
}

// Required to allow us to store a C pointer in the static LAB_FLASH_INFO
unsafe impl Sync for FlashInfo {}

/// Information about One ROM Lab's runtime state.
#[repr(C)]
pub struct RamInfo {
    pub magic: [u8; 4],
    pub rom_data: *const core::ffi::c_void,
    pub rpc_cmd_channel: *const core::ffi::c_void,
    pub rpc_rsp_channel: *const core::ffi::c_void,
    pub rpc_cmd_channel_size: u16,
    pub rpc_rsp_channel_size: u16,
    pub reserved: [u8; 236],
}
