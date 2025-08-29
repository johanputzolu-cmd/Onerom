//! One ROM Lab firmware

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_executor::main as embassy_main;
use embassy_stm32::gpio::Flex;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;

mod checksum;
mod error;
mod rom;

pub use checksum::{checksum, identify_rom};
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

    loop {
        // Read the ROM
        let mut buf = [0u8; 8192];
        info!("Reading ROM...");
        let start = Instant::now();
        if let Err(e) = rom.read(&mut buf).await {
            panic!("Failed to read ROM: {e:?}");
        }
        let end = Instant::now();

        let time_taken = end - start;
        info!("Read took {:?}", time_taken);

        // Output checksums
        let sum8: u8 = checksum(&buf);
        debug!("8-bit checksum:  {:#04x}", sum8);
        let sum16: u16 = checksum(&buf);
        debug!("16-bit checksum: {:#06x}", sum16);
        let sum32: u32 = checksum(&buf);
        debug!("32-bit checksum: {:#08x}", sum32);

        log_rom_info(sum32);

        dump_rom_data(&buf);

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn log_rom_info(sum32: u32) {
    let match_count = identify_rom(sum32).count();

    match match_count {
        0 => info!("Unknown ROM ({:#010X})", sum32),
        1 => {
            let rom = identify_rom(sum32).next().unwrap();
            info!("Identified ROM ({:#010X}): {} {}", sum32, rom.name(), rom.part());
        }
        _ => {
            info!("Multiple ROM matches ({:#010X}):", sum32);
            for rom in identify_rom(sum32) {
                info!("  - {} {}", rom.name(), rom.part());
            }
        }
    }
}

fn dump_rom_data(buf: &[u8]) {
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
