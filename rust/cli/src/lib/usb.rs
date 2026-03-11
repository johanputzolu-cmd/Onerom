// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! USB device enumeration and transport primitives.
//!
//! Handles discovery of connected One ROM Fire (RP2350) devices via the
//! PICOBOOT protocol.

#[allow(unused_imports)]
use log::{debug, warn};
use picoboot::{Picoboot, Reader as PicobootReader, Target, usb::Timeouts};
use sdrr_fw_parser::Parser;
use std::time::Duration;

use crate::Error;
use crate::{Device, DeviceState};

/// Flash start address on RP2350.
pub const FLASH_BASE: u32 = 0x1000_0000;
pub const RAM_BASE: u32 = 0x2000_0000;

/// Size of the One ROM metadata region to read from flash.
pub const FLASH_READ_SIZE_KB: u32 = 64;
pub const FLASH_READ_SIZE_BYTES: u32 = FLASH_READ_SIZE_KB * 1024;

/// Enumerate all connected One ROM Fire (RP2350) devices.
///
/// Returns an empty Vec rather than an error if no devices are found.
pub async fn enumerate_devices() -> Result<Vec<Device>, Error> {
    let fire_targets = [Target::Rp2350];
    let device_infos = Picoboot::list_devices(Some(&fire_targets))
        .await
        .map_err(|e| Error::Usb(e.to_string()))?;

    let mut devices = Vec::new();
    for info in device_infos {
        debug!(
            "Found Fire device: {:04x}:{:04x} bus {} addr {}",
            info.vendor_id(),
            info.product_id(),
            info.bus_id(),
            info.device_address(),
        );

        let mut device = Device {
            vid: info.vendor_id(),
            pid: info.product_id(),
            bus_id: info.bus_id().to_owned(),
            address: info.device_address(),
            serial: info.serial_number().map(str::to_owned),
            device_info: info,
            onerom: None,
            state: DeviceState::Unknown,
        };

        if let Err(e) = read_device_info(&mut device).await {
            warn!("Failed to read flash header on {device:?}: {e}");
        }

        devices.push(device);
    }

    Ok(devices)
}

async fn get_picoboot(device: &Device) -> Result<Picoboot, Error> {
    let mut picoboot = Picoboot::new(device.device_info.clone())
        .await
        .map_err(|e| Error::Usb(e.to_string()))?;

    picoboot.set_timeouts(Timeouts {
        endpoint: Duration::from_millis(500),
        ..Timeouts::default()
    });

    Ok(picoboot)
}

/// Read the first 64KB from flash on a One ROM Fire device.
///
/// Connects to the device via PICOBOOT, reads from the flash start address,
/// and returns the raw bytes. The caller is responsible for parsing the
/// contents.
pub async fn read_device_info(device: &mut Device) -> Result<(), Error> {
    debug!("Reading {FLASH_READ_SIZE_KB}KB from {FLASH_BASE:#010x} on {device}");

    let picoboot = get_picoboot(device).await?;
    let mut reader = PicobootReader::new(picoboot).await.map_err(Error::Usb)?;

    // Parse the flash to get the device information
    let mut parser = Parser::with_base_flash_address(&mut reader, FLASH_BASE, RAM_BASE);
    let onerom = parser.parse().await;
    device.update_onerom(onerom);

    Ok(())
}

/// What state One ROM should be rebooted into
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootMode {
    /// Stopped is bootloader/BOOTSEL mode
    Stopped,
    /// Running is One ROM in byte serving mode
    Running,
}

impl std::fmt::Display for RebootMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebootMode::Stopped => write!(f, "stopped"),
            RebootMode::Running => write!(f, "running"),
        }
    }
}

impl From<RebootMode> for picoboot::RebootType {
    fn from(mode: RebootMode) -> Self {
        match mode {
            RebootMode::Stopped => picoboot::RebootType::Bootsel {
                disable_msd: true,
                disable_picoboot: false,
            },
            RebootMode::Running => picoboot::RebootType::Normal,
        }
    }
}

/// Reboot the chosen One ROM
pub async fn reboot(device: &Device, mode: RebootMode) -> Result<(), Error> {
    let mut picoboot = get_picoboot(device).await?;

    let reboot_type = mode.into();

    picoboot
        .reboot(reboot_type, Duration::from_millis(500))
        .await
        .map_err(|e| Error::Usb(e.to_string()))
}

// A valid One ROM MCU memory region
struct MemoryRegion {
    _name: &'static str,
    start: u32,
    len: u32,
}

impl MemoryRegion {
    const fn new(name: &'static str, start: u32, len: u32) -> Self {
        Self {
            _name: name,
            start,
            len,
        }
    }

    fn contains(&self, address: u32, length: u32) -> bool {
        address >= self.start && length <= self.len && address - self.start <= self.len - length
    }
}

const VALID_REGIONS: &[MemoryRegion] = &[
    // 2MB of flash
    MemoryRegion::new("Flash", 0x1000_0000, 0x0020_0000),
    // 520KB of SRAM
    MemoryRegion::new("SRAM", 0x2000_0000, 0x0008_2000),
    // 32KB of Boot ROM
    MemoryRegion::new("ROM", 0x0000_0000, 0x0000_8000),
    // 512KB of live ROM data
    MemoryRegion::new("Live", 0x9000_0000, 0x0008_0000),
];

fn check_memory_range(address: u32, length: u32) -> Result<(), Error> {
    VALID_REGIONS
        .iter()
        .any(|r| r.contains(address, length))
        .then_some(())
        .ok_or(Error::InvalidMemoryRange(address, length))
}

/// Read bytes from device memory
pub async fn read_memory(device: &Device, address: u32, length: u32) -> Result<Vec<u8>, Error> {
    check_memory_range(address, length)?;

    let mut picoboot = get_picoboot(device).await?;

    picoboot
        .flash_read(address, length)
        .await
        .map_err(|e| Error::Usb(e.to_string()))
}
