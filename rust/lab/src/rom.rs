//! One ROM Lab - ROM handling

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use alloc::vec::Vec;
use embassy_stm32::gpio::{Flex, Pull, Speed};
use embassy_time::{Duration, Instant};

use crate::database::{checksum, identify_rom, sha1_digest};
use crate::{CsActive, RomEntry, RomType};

#[derive(Debug)]
pub struct Id {
    rom_type: RomType,
    sum: u32,
    sha1: [u8; 20],
}

impl Default for Id {
    fn default() -> Self {
        Self {
            rom_type: RomType::Type2364 { cs: CsActive::Low },
            sum: 0,
            sha1: [0u8; 20],
        }
    }
}

impl Id {
    pub fn rom_type(&self) -> RomType {
        self.rom_type
    }

    pub fn sum(&self) -> u32 {
        self.sum
    }

    pub fn sha1(&self) -> &[u8; 20] {
        &self.sha1
    }
}

#[derive(Debug, Default)]
pub struct Matches {
    good: Vec<&'static RomEntry>,
    bad: Vec<(&'static RomEntry, RomType)>,
    ids: [Id; RomType::all().len()],
}

pub struct Rom {
    address: AddressLines,
    data: DataLines,
    buf: [u8; 1 << AddressLines::NUM_ADDR_LINES],
    matches: Option<Matches>,
    last_read_duration: Option<Duration>,
}

impl Rom {
    /// Creates a new 2364 ROM object
    pub fn new(
        addr_pins: [Flex<'static>; AddressLines::NUM_ADDR_LINES],
        data_pins: [Flex<'static>; DataLines::NUM_DATA_LINES],
    ) -> Self {
        Self {
            address: AddressLines { address: addr_pins },
            data: DataLines::new(data_pins),
            buf: [0u8; 1 << AddressLines::NUM_ADDR_LINES],
            matches: None,
            last_read_duration: None,
        }
    }

    pub fn init(&mut self) {
        self.address.init();
        self.data.init();
    }

    async fn read_fast(&mut self) {
        let max_addr = 1 << AddressLines::NUM_ADDR_LINES;
        assert!(self.buf.len() == max_addr);

        let start = Instant::now();

        // Now read the ROM
        for ii in 0..max_addr {
            self.address.set(ii);
            self.buf[ii] = self.data.read();
        }
        self.address.init();

        let end = Instant::now();
        self.last_read_duration = Some(end - start);
    }

    #[allow(dead_code)]
    async fn read_slow(&mut self) {
        let max_addr = 1 << AddressLines::NUM_ADDR_LINES;
        assert!(self.buf.len() == max_addr);

        let start = Instant::now();

        // Now read the ROM
        for ii in 0..max_addr {
            self.address.set(ii);
            self.buf[ii] = self.data.read();
            self.address.init();
        }
        self.address.init();

        let end = Instant::now();
        self.last_read_duration = Some(end - start);
    }

    fn id(&mut self) {
        // Scratch buffer
        let mut buf = [0u8; 8192];

        let mut matches = Matches::default();

        for (ii, rom_type) in RomType::all().iter().enumerate() {
            // Build a temporary copy of the ROM data, based on this particular
            // ROM type and CS behaviour
            let size = rom_type.size();
            #[allow(clippy::needless_range_loop)]
            for jj in 0..size {
                let addr = jj | rom_type.cs_mask();
                buf[jj] = self.buf[addr];
            }

            // Now use this copy to get the checksum/SHA1 digest
            let sum: u32 = checksum(&buf[0..size]);
            let sha1 = sha1_digest(&buf[0..size]);
            let (mut good, mut bad) = identify_rom(rom_type, sum, sha1);

            matches.good.append(&mut good);
            matches.bad.append(&mut bad);
            matches.ids[ii] = Id {
                rom_type: *rom_type,
                sum,
                sha1,
            };
        }

        self.matches = Some(matches);
    }

    /// Reads any connected ROM and detects any matches.
    pub async fn detect(&mut self) {
        self.read_fast().await;
        self.id();
    }

    /// Gets last read duration
    pub fn last_read_duration(&self) -> Option<Duration> {
        self.last_read_duration
    }

    /// Returns good matches
    pub fn good_matches(&self) -> Option<&Vec<&'static RomEntry>> {
        self.matches.as_ref().map(|m| &m.good)
    }

    /// Returns bad matches - those where SHA1 or checksum matches, but the
    /// ROM type didn't
    pub fn bad_matches(&self) -> Option<&Vec<(&'static RomEntry, RomType)>> {
        self.matches.as_ref().map(|m| &m.bad)
    }

    /// Returns IDs of various ROM types
    pub fn ids(&self) -> Option<&[Id; RomType::all().len()]> {
        self.matches.as_ref().map(|m| &m.ids)
    }
}

pub struct AddressLines {
    // Array of GPIOs corresponding to A0, A2, ... A13.
    // A13 is the address line used for 2364's CS line.
    address: [Flex<'static>; Self::NUM_ADDR_LINES],
}

impl AddressLines {
    pub const NUM_ADDR_LINES: usize = 14;

    fn init(&mut self) {
        for pin in self.address.iter_mut() {
            pin.set_as_input(Pull::Down);
        }
    }

    #[inline]
    fn set(&mut self, address: usize) {
        assert!(address < (1 << Self::NUM_ADDR_LINES));

        // Set address pins as outputs and drive them
        for (i, pin) in self.address.iter_mut().enumerate() {
            pin.set_as_output(Speed::High);
            if address & (1 << i) != 0 {
                pin.set_high();
            } else {
                pin.set_low();
            }
        }
    }

    #[allow(dead_code)]
    fn clear(&mut self) {
        self.init()
    }
}

pub struct DataLines {
    data: [Flex<'static>; Self::NUM_DATA_LINES],
}

impl DataLines {
    pub const NUM_DATA_LINES: usize = 8;

    fn new(data_pins: [Flex<'static>; Self::NUM_DATA_LINES]) -> Self {
        Self { data: data_pins }
    }

    fn pins(&self) -> &[Flex<'static>] {
        &self.data
    }

    fn pins_mut(&mut self) -> &mut [Flex<'static>] {
        &mut self.data
    }

    fn init(&mut self) {
        for pin in self.pins_mut() {
            pin.set_as_input(Pull::None);
        }
    }

    fn read(&self) -> u8 {
        let mut value = 0u8;
        for (i, pin) in self.pins().iter().enumerate() {
            if pin.is_high() {
                value |= 1 << i;
            }
        }
        value
    }
}
