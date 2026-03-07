// Copyright (c) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT licence

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use embassy_executor::Spawner;
use embassy_executor::main as embassy_main;
use embassy_rp::{clocks::ClockConfig, config::Config};
use embassy_time::Timer;

use embedded_alloc::LlffHeap as Heap;
use panic_rtt_target as _;

mod error;
mod hw;
mod logs;
mod rom;

pub use error::Error;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_main]
async fn main(_spawner: Spawner) -> ! {
    // Initialize the heap allocator
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    // Init logging
    logs::init_rtt();

    info!("-----");
    info!("One ROM Lab NEW v{}", PKG_VERSION);
    info!("Copyright (c) 2026 Piers Finlayson");

    info!("-----");
    debug!("RP2350 target");

    // Initialize peripherals with clocks set to 150MHz
    let mut config = Config::default();
    let clocks = ClockConfig::system_freq(150_000_000)
        .expect("Failed to configure clocks");
    config.clocks = clocks;
    let _p = embassy_rp::init(config);

    debug!("Clocks configured to 150MHz");

    // Set up the LED
    let led_pins = hw::led_pins();
    let [mut led] = led_pins;

    // Flash LED to show we're alive
    led.set_as_output();
    for _ in 0..2 {
        led.set_high();
        Timer::after_millis(200).await;
        led.set_low();
        Timer::after_millis(200).await;
    }

    // Get the other pins
    let addr_pins = hw::addr_pins();
    let data_pins = hw::data_pins();
    let [ce, oe] = hw::cs_pins();
    let [byte_n] = hw::special_pins();

    // Create the ROM object
    let mut rom = rom::Rom27C400::new(addr_pins, data_pins, ce, oe, byte_n);
    rom.init();

    debug!("-----");

    loop {
        info!("Reading {} ...", rom::Rom27C400::type_as_str());

        let ((sha1_8, csum_8, ts_fail_8), (sha1_16, csum_16, ts_fail_16)) = rom.read();

        info!("8-bit  SHA1: {} checksum: {:#010X}", hex::encode(sha1_8), csum_8);
        info!("16-bit SHA1: {} checksum: {:#010X}", hex::encode(sha1_16), csum_16);
        info!("Match: {}", sha1_8 == sha1_16 && csum_8 == csum_16);
        info!("Tristate failures: 8-bit: {} 16-bit: {}", ts_fail_8, ts_fail_16);
        info!("-----");
        embassy_time::Timer::after_secs(1).await;
    }
}
