//! One ROM Lab - GPIO handling

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use embassy_stm32::gpio::{Flex, Pull, Speed};

use crate::error::Error;
use crate::CsActive;

pub struct Rom {
    address: AddressLines,
    data: DataLines,
}

impl Rom {
    /// Creates a new 2364 ROM
    pub fn new_2364(
        addr_pins: [Flex<'static>; ADDR_LINES_2364],
        cs: Cs,
        data_pins: [Flex<'static>; DATA_LINES],
    ) -> Self {
        Self {
            address: AddressLines::Rom2364(Address2364 {
                address: addr_pins,
                cs: [cs],
            }),
            data: DataLines::new(data_pins),
        }
    }

    /// Creates a new 2332 ROM
    #[allow(dead_code)]
    pub fn new_2332(
        addr_pins: [Flex<'static>; ADDR_LINES_2332],
        cs1: Cs,
        cs2: Cs,
        data_pins: [Flex<'static>; DATA_LINES],
    ) -> Self {
        Self {
            address: AddressLines::Rom2332(Address2332 {
                address: addr_pins,
                cs: [cs1, cs2],
            }),
            data: DataLines::new(data_pins),
        }
    }

    /// Creates a new 2316 ROM
    #[allow(dead_code)]
    pub fn new_2316(
        addr_pins: [Flex<'static>; ADDR_LINES_2316],
        cs1: Cs,
        cs2: Cs,
        cs3: Cs,
        data_pins: [Flex<'static>; DATA_LINES],
    ) -> Self {
        Self {
            address: AddressLines::Rom2316(Address2316 {
                address: addr_pins,
                cs: [cs1, cs2, cs3],
            }),
            data: DataLines::new(data_pins),
        }
    }

    pub fn init(&mut self) {
        self.address.init();
        self.data.init();
    }

    pub fn size(&self) -> u32 {
        1 << self.address.bits()
    }

    /// Get the type of ROM address by these address lines
    pub fn rom_type(&self) -> &str {
        match self.address {
            AddressLines::Rom2364(_) => "2364",
            AddressLines::Rom2332(_) => "2332",
            AddressLines::Rom2316(_) => "2316",
        }
    }

    /// Read the entire ROM into the provided buffer, with CS active the
    /// whole time.  This is the fastest way to read the ROM, but is not a
    /// full test of the ROM itself.
    pub async fn read_single_cs(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let size = self.size();
        if buf.len() < size as usize {
            return Err(Error::Buffer);
        }

        // Now read the ROM
        self.address.set_cs(true);
        for ii in 0..size {
            self.address.set_address(ii)?;
            buf[ii as usize] = self.data.read();
        }
        self.address.set_cs(false);
        Ok(())
    }

    /// Read the entire ROM into the provided buffer, deasserting CS
    /// between reads.
    pub async fn read_toggle_cs(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let size = self.size();
        if buf.len() < size as usize {
            return Err(Error::Buffer);
        }

        // Now read the ROM
        for ii in 0..size {
            self.address.set(ii, true)?;
            buf[ii as usize] = self.data.read();
        }
        self.address.set_cs(false);
        Ok(())
    }
}

pub struct Cs {
    pin: Flex<'static>,
    active: CsActive,
}

impl Cs {
    pub fn new(pin: Flex<'static>, active: CsActive) -> Self {
        Cs { pin, active }
    }

    fn init(&mut self) {
        self.pin.set_as_output(Speed::High);
        self.set_inactive();
    }

    fn set_active(&mut self) {
        match self.active {
            CsActive::High => self.pin.set_high(),
            CsActive::Low => self.pin.set_low(),
        }
    }

    fn set_inactive(&mut self) {
        match self.active {
            CsActive::High => self.pin.set_low(),
            CsActive::Low => self.pin.set_high(),
        }
    }
}

pub const ADDR_LINES_2364: usize = 13;
pub const ADDR_LINES_2332: usize = 12;
pub const ADDR_LINES_2316: usize = 11;
pub const CS_LINES_2364: usize = 1;
pub const CS_LINES_2332: usize = 2;
pub const CS_LINES_2316: usize = 3;

pub struct Address2364 {
    address: [Flex<'static>; ADDR_LINES_2364],
    cs: [Cs; CS_LINES_2364],
}

pub struct Address2332 {
    address: [Flex<'static>; ADDR_LINES_2332],
    cs: [Cs; CS_LINES_2332],
}

pub struct Address2316 {
    address: [Flex<'static>; ADDR_LINES_2316],
    cs: [Cs; CS_LINES_2316],
}

pub enum AddressLines {
    Rom2364(Address2364),
    #[allow(dead_code)]
    Rom2332(Address2332),
    #[allow(dead_code)]
    Rom2316(Address2316),
}

impl AddressLines {
    fn bits(&self) -> usize {
        self.pins().len()
    }

    #[allow(dead_code)]
    fn cs_pins(&self) -> &[Cs] {
        match self {
            AddressLines::Rom2364(a) => &a.cs,
            AddressLines::Rom2332(a) => &a.cs,
            AddressLines::Rom2316(a) => &a.cs,
        }
    }

    fn cs_pins_mut(&mut self) -> &mut [Cs] {
        match self {
            AddressLines::Rom2364(a) => &mut a.cs,
            AddressLines::Rom2332(a) => &mut a.cs,
            AddressLines::Rom2316(a) => &mut a.cs,
        }
    }

    fn pins(&self) -> &[Flex<'static>] {
        match self {
            AddressLines::Rom2364(a) => &a.address,
            AddressLines::Rom2332(a) => &a.address,
            AddressLines::Rom2316(a) => &a.address,
        }
    }

    fn pins_mut(&mut self) -> &mut [Flex<'static>] {
        match self {
            AddressLines::Rom2364(a) => &mut a.address,
            AddressLines::Rom2332(a) => &mut a.address,
            AddressLines::Rom2316(a) => &mut a.address,
        }
    }

    fn init(&mut self) {
        for pin in self.pins_mut() {
            pin.set_as_input(Pull::None);
        }

        for pin in self.cs_pins_mut() {
            pin.init();
        }
    }

    fn set(&mut self, address: u32, cs: bool) -> Result<(), Error> {
        self.set_address(address)?;
        self.set_cs(cs);
        Ok(())
    }

    #[allow(dead_code)]
    fn clear(&mut self) {
        self.init()
    }

    #[inline]
    fn set_address(&mut self, address: u32) -> Result<(), Error> {
        let bits = self.bits();
        if address >= (1 << bits) {
            return Err(Error::Address);
        }

        // Set address pins as outputs and drive them
        for (i, pin) in self.pins_mut().iter_mut().enumerate() {
            pin.set_as_output(Speed::High);
            if address & (1 << i) != 0 {
                pin.set_high();
            } else {
                pin.set_low();
            }
        }

        Ok(())
    }

    #[inline]
    fn set_cs(&mut self, cs: bool) {
        for cs_pin in self.cs_pins_mut() {
            if cs {
                cs_pin.set_active();
            } else {
                cs_pin.set_inactive();
            }
        }
    }
}

pub const DATA_LINES: usize = 8;
pub struct DataLines {
    data: [Flex<'static>; DATA_LINES],
}

impl DataLines {
    fn new(data_pins: [Flex<'static>; DATA_LINES]) -> Self {
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
