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
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource, Sysclk, clocks,
};
use embassy_time::{Duration, Timer};
use embedded_alloc::LlffHeap as Heap;

use panic_probe as _;

mod database;
mod error;
mod info;
mod rom;
mod types;

pub use database::Entry as RomEntry;
pub use error::Error;
pub use rom::{Id as RomId, Rom};
pub use types::{CsActive, RomType};

use info::{PKG_VERSION, LAB_RAM_INFO};

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

    info!("-----");
    info!("One ROM Lab v{}", PKG_VERSION);
    info!("Copyright (c) 2025 Piers Finlayson");

    // Set up the clocks - assume we are running on an F405RG with max clock
    // of 168MHz
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.pll_src = PllSource::HSI;
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV16,
        mul: PllMul::MUL336,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV7),
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1; // 168MHz
    config.rcc.apb1_pre = APBPrescaler::DIV4; // 42MHz (max for APB1)
    config.rcc.apb2_pre = APBPrescaler::DIV2; // 84MHz (max for APB2)

    let p = embassy_stm32::init(config);

    let clocks = clocks(&p.RCC);
    debug!("-----");
    debug!("SYSCLK: {}", clocks.sys);

    // Collate the address and data pins
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
        Flex::new(p.PC10), // 2364 CS pin, set as "A13"
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

    // Create the ROM object
    let mut rom = Rom::new(addr_pins, data_pins);
    unsafe { LAB_RAM_INFO.rom_data = rom.buf.as_ptr() as *const core::ffi::c_void; }
    rom.init();

    loop {
        // Read any connected ROM
        info!("-----");
        info!("Reading ROM...");
        rom.detect().await;
        let dur = rom.last_read_duration().unwrap();
        debug!("Read took {}us", dur.as_micros());

        // Output any good matches
        let good_matches = rom.good_matches().unwrap();
        if !good_matches.is_empty() {
            for entry in good_matches {
                log_good_rom_match(entry);
            }
        }

        // Also output any bad matches
        let bad_matches = rom.bad_matches().unwrap();
        if !bad_matches.is_empty() {
            for (entry, rom_type) in bad_matches {
                log_bad_rom_match(entry, rom_type);
            }
        }

        // If we got none of either, log why
        if good_matches.is_empty() && bad_matches.is_empty() {
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
            }
            if all_zeros_count == ids.len() {
                warn!("ROM data is all zeros - check ROM is connected");
            } else if all_ones_count == ids.len() {
                warn!("ROM data is all 0xFF - ROM may be blank or unprogrammed");
            } else {
                info!("No matches found in database - ROM information follows:");
                for id in ids {
                    if !id.all_zeros() && !id.all_ones() {
                        log_rom_id(id);
                    }
                }
            }
        }

        // Pause before restarting
        Timer::after(Duration::from_secs(3600)).await;
    }
}

fn log_good_rom_match(entry: &RomEntry) {
    info!("ROM match found:");
    info!("  Name:        {}", entry.name());
    info!("  Part:        {}", entry.part());
    info!("  Type:        {}", entry.rom_type());
    info!("  Checksum:    {:#010x}", entry.sum());
    info!("  SHA1:        {}", hex::encode(entry.sha1()));
}

fn log_bad_rom_match(entry: &RomEntry, rom_type: &RomType) {
    info!("ROM mismatch found:");
    info!("  Name:        {}", entry.name());
    info!("  Part:        {}", entry.part());
    info!("  Expected:    {}", entry.rom_type());
    info!("  Found:       {}", rom_type);
    info!("  Checksum:    {:#010x}", entry.sum());
    info!("  SHA1:        {}", hex::encode(entry.sha1()));
}

fn log_rom_id(id: &RomId) {
    info!("{}", id.rom_type().type_str());
    info!("  Chip Select: {}", id.rom_type().cs_str());
    info!("  Checksum:    {:#010x}", id.sum());
    info!("  SHA1:        {}", hex::encode(id.sha1()));
}

pub fn dump_buf(buf: &[u8]) {
    for (i, chunk) in buf.chunks(16).enumerate() {
        let addr = i * 16;

        // Pad chunk to 16 bytes for consistent formatting
        let mut line = [0u8; 16];
        line[..chunk.len()].copy_from_slice(chunk);
        let len = chunk.len();

        if len == 16 {
            debug!(
                "{:04x}:  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}",
                addr,
                line[0],
                line[1],
                line[2],
                line[3],
                line[4],
                line[5],
                line[6],
                line[7],
                line[8],
                line[9],
                line[10],
                line[11],
                line[12],
                line[13],
                line[14],
                line[15]
            );
        } else {
            // Handle partial lines
            debug!("{:04x}:  partial line, {} bytes", addr, len);
        }
    }
}
