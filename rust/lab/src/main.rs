//! One ROM Lab firmware

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_executor::main as embassy_main;
use embassy_stm32::gpio::Flex;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;

mod error;
mod rom;

pub use error::Error;
pub use rom::{Cs, CsActive, Rom};

#[embassy_main]
async fn main(_spawner: Spawner) {
    info!("One ROM Lab");

    let p = embassy_stm32::init(Default::default());

    // Collate the address and data pins, and create CS pin
    let addr_pins = [
        Flex::new(p.PC5),
        Flex::new(p.PC4),
        Flex::new(p.PC6),
        Flex::new(p.PC7),
        Flex::new(p.PC3),
        Flex::new(p.PC2),
        Flex::new(p.PC1),
        Flex::new(p.PC0),
        Flex::new(p.PC8),
        Flex::new(p.PC13),
        Flex::new(p.PC11),
        Flex::new(p.PC12),
        Flex::new(p.PC9),
    ];
    let data_pins = [
        Flex::new(p.PA7),
        Flex::new(p.PA6),
        Flex::new(p.PA5),
        Flex::new(p.PA4),
        Flex::new(p.PA3),
        Flex::new(p.PA2),
        Flex::new(p.PA1),
        Flex::new(p.PA0),
    ];
    let cs_pin = Cs::new(Flex::new(p.PC10), CsActive::Low);

    // Create the ROM
    let mut rom = Rom::new_2364(addr_pins, cs_pin, data_pins);
    rom.init();

    info!("ROM type {}, CS active low", rom.rom_type());

    // Read the ROM
    let mut buf = [0u8; 8192];
    let start = Instant::now();
    if let Err(e) = rom.read(&mut buf) {
        panic!("Failed to read ROM: {e:?}");
    }
    let end = Instant::now();

    let time_taken = end - start;
    info!("Read took {:?}", time_taken);

    let checksum = checksum_16(&buf);
    info!("16-bit checksum: {:#06x}", checksum);

    dump_rom(&buf);

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn checksum_16(data: &[u8]) -> u16 {
    let mut checksum = 0u16;
    for &byte in data {
        checksum = checksum.wrapping_add(byte as u16);
    }
    checksum
}

fn dump_rom(buf: &[u8]) {
    for (addr, chunk) in buf.chunks(16).enumerate() {
        let base_addr = addr * 16;
        if chunk.len() == 16 {
            debug!(
                "{:04x}:  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}",
                base_addr,
                chunk[0],
                chunk[1],
                chunk[2],
                chunk[3],
                chunk[4],
                chunk[5],
                chunk[6],
                chunk[7],
                chunk[8],
                chunk[9],
                chunk[10],
                chunk[11],
                chunk[12],
                chunk[13],
                chunk[14],
                chunk[15]
            );
        } else {
            debug!("{:04x}: [partial {} bytes]", base_addr, chunk.len());
        }
    }
}
