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
use embassy_time::{Duration, Instant, Timer};
use embedded_alloc::LlffHeap as Heap;

use panic_probe as _;

mod database;
mod error;
mod rom;

pub use database::{checksum, identify_rom, sha1_digest};
pub use error::Error;
pub use rom::{Cs, CsActive, Rom};

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
        info!("-----");
        info!("Reading ROM...");
        let start = Instant::now();
        if let Err(e) = rom.read_single_cs(&mut buf).await {
            panic!("Failed to read ROM: {e:?}");
        }
        let end = Instant::now();
        let time_taken = end - start;

        let mut buf2 = [0u8; 8192];
        let start2 = Instant::now();
        if let Err(e) = rom.read_toggle_cs(&mut buf2).await {
            panic!("Failed to read ROM: {e:?}");
        }
        let end2 = Instant::now();
        let time_taken2 = end2 - start2;
        debug!(
            "Reads took: single CS {}us, toggle CS {}us",
            time_taken.as_micros(),
            time_taken2.as_micros()
        );

        // Output checksums
        let sum: u32 = checksum(&buf);
        let sum2: u32 = checksum(&buf2);
        if sum2 != sum {
            warn!(
                "Data mismatch between read methods: single CS {:#010X}, toggle CS {:#010X}",
                sum, sum2
            );
        }
        debug!("32-bit checksum: {:#08x}", sum);

        let sha1 = sha1_digest(&buf);
        let buf2_sha1 = sha1_digest(&buf2);
        log_rom_info(&sha1, sum);
        if sha1 != buf2_sha1 {
            log_rom_info(&buf2_sha1, sum2);
        }

        dump_rom_data(&buf);

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn log_rom_info(sha1: &[u8; 20], sum: u32) {
    debug!(
        "Identifying ROM SHA1: {} 32-bit checksum: {:#010X}",
        hex::encode(sha1),
        sum
    );
    let rom = identify_rom(sha1, sum);
    match rom {
        None => warn!(
            "Unknown ROM Checksum: {:010x} SHA1: {}",
            sum,
            hex::encode(sha1)
        ),
        Some(rom) => info!(
            "{} {} Checksum: {:#010x} SHA1: {}",
            rom.name(),
            rom.part(),
            sum,
            hex::encode(sha1),
        ),
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
