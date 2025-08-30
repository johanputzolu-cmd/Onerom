//! One ROM Lab firmware

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_executor::main as embassy_main;
use embassy_stm32::gpio::Flex;
use embassy_stm32::rcc;
use embassy_time::{Duration, Timer};
use embedded_alloc::LlffHeap as Heap;

use panic_probe as _;

mod database;
mod error;
mod rom;
mod types;

pub use database::Entry as RomEntry;
pub use error::Error;
pub use rom::{Id as RomId, Rom};
pub use types::{CsActive, RomType};

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_main]
async fn main(_spawner: Spawner) {
    // Initialize the heap allocator
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    info!("One ROM Lab");

    // Set up the clocks - assume we are running on an F405RG with max clock
    // of 168MHz
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.pll_src = embassy_stm32::rcc::PllSource::HSI;
    config.rcc.pll = Some(embassy_stm32::rcc::Pll {
        prediv: embassy_stm32::rcc::PllPreDiv::DIV16,
        mul: embassy_stm32::rcc::PllMul::MUL336,
        divp: Some(embassy_stm32::rcc::PllPDiv::DIV2),
        divq: Some(embassy_stm32::rcc::PllQDiv::DIV7),
        divr: None,
    });
    config.rcc.sys = embassy_stm32::rcc::Sysclk::PLL1_P;
    config.rcc.ahb_pre = embassy_stm32::rcc::AHBPrescaler::DIV1; // 168MHz
    config.rcc.apb1_pre = embassy_stm32::rcc::APBPrescaler::DIV4; // 42MHz (max for APB1)
    config.rcc.apb2_pre = embassy_stm32::rcc::APBPrescaler::DIV2; // 84MHz (max for APB2)

    let p = embassy_stm32::init(config);

    let clocks = rcc::clocks(&p.RCC);
    info!("Clocks: {}", clocks);

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
        Flex::new(p.PC10),
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

    // Create the ROM
    let mut rom = Rom::new(addr_pins, data_pins);
    rom.init();

    loop {
        info!("-----");
        info!("Reading ROM...");
        rom.detect().await;
        let dur = rom.last_read_duration().unwrap();
        debug!("Read took {}us", dur.as_micros());

        let good_matches = rom.good_matches().unwrap();
        if !good_matches.is_empty() {
            info!("ROM matches found:");
            for entry in good_matches {
                log_good_rom_match(entry);
            }
        } else {
            info!("No ROM matches found");
        }

        let bad_matches = rom.bad_matches().unwrap();
        if !bad_matches.is_empty() {
            info!("ROM matches found with wrong ROM type");
            for (entry, rom_type) in bad_matches {
                log_bad_rom_match(entry, rom_type);
            }
        }

        if good_matches.is_empty() && bad_matches.is_empty() {
            info!("No matches found in database - ROM information follows:");
            let ids = rom.ids().unwrap();
            let mut all_zeros_count = 0;
            let mut all_ones_count = 0;
            for id in ids {
                if id.all_zeros() {
                    all_zeros_count += 1;
                }
                if id.all_ones() {
                    all_ones_count += 1;
                }
                if !id.all_zeros() && !id.all_ones() {
                    log_rom_id(id);
                }
            }
            if all_zeros_count == ids.len() {
                info!("- ROM images are all-zeros - is a ROM connected?");
            }
            if all_ones_count == ids.len() {
                info!("- ROM images are all-ones - is ROM empty?");
            }
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn log_good_rom_match(entry: &RomEntry) {
    info!(
        "- {} {} {} Checksum: {:#010x} SHA1: {}",
        entry.name(),
        entry.part(),
        entry.rom_type(),
        entry.sum(),
        hex::encode(entry.sha1())
    );
}

fn log_bad_rom_match(entry: &RomEntry, rom_type: &RomType) {
    info!(
        "- {} {} Database: {} Found: {} Checksum: {:#010x} SHA1: {}",
        entry.name(),
        entry.part(),
        entry.rom_type(),
        rom_type,
        entry.sum(),
        hex::encode(entry.sha1())
    );
}

fn log_rom_id(id: &RomId) {
    info!(
        "- {} Checksum: {:#010x} SHA1: {}",
        id.rom_type(),
        id.sum(),
        hex::encode(id.sha1())
    );
}
