// Copyright (c) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

// Expansion notes:
//
// This module currently contains a single concrete type, Rom27C400, which is
// a one-off due to its BYTE# pin and 16-bit data bus.
//
// Future 8-bit EPROMs (27C128/256/512/C010/C020/C040) will be handled by a
// single RomEprom8 type parameterised by address line count - all share
// identical read logic (CE, OE, N address lines, 8 data lines).
//
// Mask ROMs (23xxx series) will require a separate RomMask type due to
// configurable CS line polarity and potentially multiple CS lines.

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use core::num::Wrapping;
use embassy_rp::gpio::{Flex, Pull};
use sha1::{Digest, Sha1};

/// (sha1 digest, wrapping 32-bit checksum, tristate failure count)
pub type DigestResult = ([u8; 20], u32, u32);

/// (8-bit byte mode result, 16-bit word mode result)
pub type ReadResult = (DigestResult, DigestResult);

struct ChecksumState(Wrapping<u32>);

impl ChecksumState {
    fn new() -> Self {
        Self(Wrapping(0))
    }

    #[inline]
    fn update(&mut self, byte: u8) {
        self.0 = self.0 + Wrapping(byte as u32);
    }

    fn finish(self) -> u32 {
        self.0 .0
    }
}

#[derive(Clone, Copy)]
enum BitMode {
    Eight,
    Sixteen,
}

/// 27C400 40-pin 512KB EPROM reader.
///
/// addr[0] = A-1 (byte select address line)
/// addr[1..19] = A0..A17
/// data[0..7] = D0-D7 (low byte)
/// data[8..15] = D8-D15 (high byte)
pub struct Rom27C400 {
    addr: [Flex<'static>; 19],
    data: [Flex<'static>; 16],
    ce: Flex<'static>,
    oe: Flex<'static>,
    byte_n: Flex<'static>,
}

impl Rom27C400 {
    // Empirically determined read delay cycles for stable reads at 150MHz
    // clock.  The CS inactive delay needs a long time due to weak-pull downs,
    // and any capacitance/inductance of the test setup (e.g. pogo pins).
    const READ_DELAY_CYCLES_8_BIT: u32 = 12;
    const READ_DELAY_CYCLES_16_BIT: u32 = 8;
    const CS_INACTIVE_DELAY_CYCLES: u32 = 200;

    pub fn new(
        addr: [Flex<'static>; 19],
        data: [Flex<'static>; 16],
        ce: Flex<'static>,
        oe: Flex<'static>,
        byte_n: Flex<'static>,
    ) -> Self {
        Self { addr, data, ce, oe, byte_n }
    }

    pub const fn type_as_str() -> &'static str {
        "27C400"
    }

    pub fn init(&mut self) {
        for pin in self.addr.iter_mut() {
            pin.set_as_output();
            pin.set_low();
        }
        for pin in self.data.iter_mut() {
            pin.set_pull(Pull::Down);
            pin.set_as_input();
        }
        self.ce.set_as_output();
        self.ce.set_high();
        self.oe.set_as_output();
        self.oe.set_high();
        self.byte_n.set_as_output();
        self.byte_n.set_high();
    }

    #[inline(always)]
    fn set_addr(&mut self, addr: usize) {
        for (i, pin) in self.addr.iter_mut().enumerate() {
            if addr & (1 << i) != 0 {
                pin.set_high();
            } else {
                pin.set_low();
            }
        }
    }

    #[inline(always)]
    fn read_low_byte(&self) -> u8 {
        let mut val = 0u8;
        for (i, pin) in self.data[0..8].iter().enumerate() {
            if pin.is_high() {
                val |= 1 << i;
            }
        }
        val
    }

    #[inline(always)]
    fn read_high_byte(&self) -> u8 {
        let mut val = 0u8;
        for (i, pin) in self.data[8..16].iter().enumerate() {
            if pin.is_high() {
                val |= 1 << i;
            }
        }
        val
    }

    // Inline always
    #[inline(always)]
    fn read_delay_8_bit() {
        cortex_m::asm::delay(Self::READ_DELAY_CYCLES_8_BIT);
    }

    #[inline(always)]
    fn read_delay_16_bit() {
        cortex_m::asm::delay(Self::READ_DELAY_CYCLES_16_BIT);
    }

    #[inline(always)]
    fn cs_inactive_delay() {
        cortex_m::asm::delay(Self::CS_INACTIVE_DELAY_CYCLES);
    }

    // 8-bit byte mode: 524288 byte addresses, addr[0]=A-1 is LSB.
    // Even addresses (A-1=0) yield low bytes, odd (A-1=1) yield high bytes,
    // producing the same byte stream as word mode.
    // Don't inline so function can be analysed in map/dis files. 
    #[inline(never)]
    fn read_mode(&mut self, mode: BitMode, sha: &mut Sha1, csum: &mut ChecksumState) -> u32 {
        let mut failures = 0u32;

        let (addr_count, addr_shift, delay): (usize, usize, fn()) = match mode {
            BitMode::Eight => (1 << 19, 0, Self::read_delay_8_bit),
            BitMode::Sixteen => (1 << 18, 1, Self::read_delay_16_bit),
        };

        match mode {
            BitMode::Eight => self.byte_n.set_low(),
            BitMode::Sixteen => self.byte_n.set_high(),
        }
        self.ce.set_low();
        self.oe.set_low();

        for addr in 0..addr_count {
            self.set_addr(addr << addr_shift);
            delay();

            let lo = self.read_low_byte();
            sha.update([lo]);
            csum.update(lo);

            if let BitMode::Sixteen = mode {
                let hi = self.read_high_byte();
                sha.update([hi]);
                csum.update(hi);
            }

            // Test OE tristate
            self.oe.set_high();
            Self::cs_inactive_delay();
            let oe_fail = self.read_low_byte() != 0
                || matches!(mode, BitMode::Sixteen) && self.read_high_byte() != 0;
            if oe_fail {
                failures += 1;
            }
            self.oe.set_low();

            // Test CE tristate
            self.ce.set_high();
            Self::cs_inactive_delay();
            let ce_fail = self.read_low_byte() != 0
                || matches!(mode, BitMode::Sixteen) && self.read_high_byte() != 0;
            if ce_fail {
                failures += 1;
            }
            self.ce.set_low();
        }

        self.oe.set_high();
        self.ce.set_high();
        self.byte_n.set_high();

        failures
    }

    /// Reads the ROM in both 8-bit byte mode and 16-bit word mode.
    ///
    /// Returns ((sha1_8bit, checksum_8bit, failures_8bit), (sha1_16bit, checksum_16bit, failures_16bit)).
    /// If the ROM and wiring are correct both SHA1s and checksums will be identical,
    /// and both failure counts will be zero.
    pub fn read(&mut self) -> ReadResult {
        let mut sha_8 = Sha1::new();
        let mut csum_8 = ChecksumState::new();
        let failures_8 = self.read_mode(BitMode::Eight, &mut sha_8, &mut csum_8);
        let mut sha1_8 = [0u8; 20];
        sha1_8.copy_from_slice(&sha_8.finalize());

        let mut sha_16 = Sha1::new();
        let mut csum_16 = ChecksumState::new();
        let failures_16 = self.read_mode(BitMode::Sixteen, &mut sha_16, &mut csum_16);
        let mut sha1_16 = [0u8; 20];
        sha1_16.copy_from_slice(&sha_16.finalize());

        ((sha1_8, csum_8.finish(), failures_8), (sha1_16, csum_16.finish(), failures_16))
    }
}