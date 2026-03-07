// Copyright (c) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

use embassy_rp::gpio::Flex;
use embassy_rp::peripherals::*;

#[cfg(feature = "fire-40-a")]
pub fn addr_pins() -> [Flex<'static>; 19] {
    [
        Flex::new(unsafe { PIN_37::steal() }),
        Flex::new(unsafe { PIN_36::steal() }),
        Flex::new(unsafe { PIN_35::steal() }),
        Flex::new(unsafe { PIN_34::steal() }),
        Flex::new(unsafe { PIN_33::steal() }),
        Flex::new(unsafe { PIN_32::steal() }),
        Flex::new(unsafe { PIN_31::steal() }),
        Flex::new(unsafe { PIN_30::steal() }),
        Flex::new(unsafe { PIN_29::steal() }),
        Flex::new(unsafe { PIN_27::steal() }),
        Flex::new(unsafe { PIN_26::steal() }),
        Flex::new(unsafe { PIN_25::steal() }),
        Flex::new(unsafe { PIN_24::steal() }),
        Flex::new(unsafe { PIN_23::steal() }),
        Flex::new(unsafe { PIN_22::steal() }),
        Flex::new(unsafe { PIN_21::steal() }),
        Flex::new(unsafe { PIN_20::steal() }),
        Flex::new(unsafe { PIN_19::steal() }),
        Flex::new(unsafe { PIN_28::steal() }),
    ]
}

#[cfg(feature = "fire-40-a")]
pub fn cs_pins() -> [Flex<'static>; 2] {
    [
        Flex::new(unsafe { PIN_16::steal() }),
        Flex::new(unsafe { PIN_17::steal() }),
    ]
}

#[cfg(feature = "fire-40-a")]
pub fn special_pins() -> [Flex<'static>; 1] {
    [
        Flex::new(unsafe { PIN_18::steal() }),
    ]
}

#[cfg(feature = "fire-40-a")]
pub fn data_pins() -> [Flex<'static>; 16] {
    [
        Flex::new(unsafe { PIN_0::steal() }),
        Flex::new(unsafe { PIN_1::steal() }),
        Flex::new(unsafe { PIN_2::steal() }),
        Flex::new(unsafe { PIN_3::steal() }),
        Flex::new(unsafe { PIN_4::steal() }),
        Flex::new(unsafe { PIN_5::steal() }),
        Flex::new(unsafe { PIN_6::steal() }),
        Flex::new(unsafe { PIN_7::steal() }),
        Flex::new(unsafe { PIN_8::steal() }),
        Flex::new(unsafe { PIN_9::steal() }),
        Flex::new(unsafe { PIN_10::steal() }),
        Flex::new(unsafe { PIN_11::steal() }),
        Flex::new(unsafe { PIN_12::steal() }),
        Flex::new(unsafe { PIN_13::steal() }),
        Flex::new(unsafe { PIN_14::steal() }),
        Flex::new(unsafe { PIN_15::steal() }),
    ]
}

#[cfg(feature = "fire-40-a")]
pub fn led_pins() -> [Flex<'static>; 1] {
    [
        Flex::new(unsafe { PIN_42::steal() }),
    ]
}