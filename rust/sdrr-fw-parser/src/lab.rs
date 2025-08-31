//! sdrr-fw-parser - One ROM Lab object handling

// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use deku::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{Reader, STM32F4_FLASH_BASE, STM32F4_RAM_BASE, Source, read_string_at_ptr};

/// Container for both the parsed (flash) firmware information and (RAM)
/// runtime information.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OneRomLab {
    pub flash: Option<LabFlash>,
    /// Runtime information
    pub ram: Option<LabRam>,
}

/// Main One ROM Lab flash information data structure.
///
/// Reflects `onerom_lab::FlashInfo`
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LabFlash {
    pub major_version: Option<String>,
    pub minor_version: Option<String>,
    pub patch_version: Option<String>,
    pub build_number: Option<String>,
    pub mcu: Option<String>,
    pub hw_rev: Option<String>,
    pub rtt_ptr: u32,
}

/// Main One ROM Lab RAM information data structure.
///
/// Reflects `onerom_lab::RamInfo`
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LabRam {}

#[derive(Debug, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"ONEL")]
// Used internally to construct [`LabFlash`]
//
// Reflects `onerom_lab::FlashInfo`
struct LabFlashInt {
    #[deku(endian = "little")]
    major_version_ptr: u32,
    #[deku(endian = "little")]
    minor_version_ptr: u32,
    #[deku(endian = "little")]
    patch_version_ptr: u32,
    #[deku(endian = "little")]
    build_number_ptr: u32,
    #[deku(endian = "little")]
    mcu_ptr: u32,
    #[deku(endian = "little")]
    hw_rev_ptr: u32,
    #[deku(endian = "little")]
    rtt_ptr: u32,
    reserved: [u8; 200],
}

impl LabFlashInt {
    const SIZE: usize = 256;
    const OFFSET: usize = 0x200;
    const _SOURCE: Source = Source::Ram;

    const fn size() -> usize {
        Self::SIZE
    }

    const fn offset() -> usize {
        Self::OFFSET
    }

    const fn _source() -> Source {
        Self::_SOURCE
    }

    fn parse_and_validate(data: &[u8]) -> Result<Self, String> {
        if data.len() < Self::size() {
            return Err("Lab flash data too short".into());
        }

        let (_, info) = Self::from_bytes((data, 0))
            .map_err(|e| format!("Failed to parse Lab flash data: {}", e))?;

        Ok(info)
    }
}

#[derive(Debug, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"onel")]
// Used internally to construct [`LabFlash`]
//
// Reflects `onerom_lab::FlashInfo`
struct LabRamInt {
    reserved: [u8; 252],
}

impl LabRamInt {
    const SIZE: usize = 256;
    const OFFSET: usize = 0;
    const _SOURCE: Source = Source::Ram;

    const fn size() -> usize {
        Self::SIZE
    }

    const fn offset() -> usize {
        Self::OFFSET
    }

    const fn _source() -> Source {
        Self::_SOURCE
    }

    fn parse_and_validate(data: &[u8]) -> Result<Self, String> {
        if data.len() < Self::size() {
            return Err("Lab RAM data too short".into());
        }

        let (_, info) = Self::from_bytes((data, 0))
            .map_err(|e| format!("Failed to parse Lab RAM data: {}", e))?;

        Ok(info)
    }
}

pub struct LabParser<R: Reader> {
    reader: R,
    base_flash_address: u32,
    base_ram_address: u32,
}

impl<R: Reader> LabParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            base_flash_address: STM32F4_FLASH_BASE,
            base_ram_address: STM32F4_RAM_BASE,
        }
    }

    pub fn with_base_flash_address(
        reader: R,
        base_flash_address: u32,
        base_ram_address: u32,
    ) -> Self {
        Self {
            reader,
            base_flash_address,
            base_ram_address,
        }
    }

    async fn retrieve_flash_info(&mut self) -> Result<LabFlashInt, String> {
        let addr = self.base_flash_address + LabFlashInt::offset() as u32;
        let mut buf = [0u8; LabFlashInt::size()];
        self.reader
            .read(addr, &mut buf)
            .await
            .map_err(|_| "Failed to read Lab Flash info")?;
        LabFlashInt::parse_and_validate(&buf)
    }

    async fn retrieve_ram_info(&mut self) -> Result<LabRamInt, String> {
        let addr = self.base_ram_address + LabRamInt::offset() as u32;
        let mut buf = [0u8; LabRamInt::size()];
        self.reader
            .read(addr, &mut buf)
            .await
            .map_err(|_| "Failed to read Lab RAM info")?;
        LabRamInt::parse_and_validate(&buf)
    }

    pub async fn detect(&mut self) -> bool {
        self.retrieve_flash_info().await.is_ok()
    }

    pub async fn parse(&mut self) -> OneRomLab {
        let flash = match self.parse_flash().await {
            Ok(info) => Some(info),
            Err(e) => {
                warn!("Failed to parse flash: {e}");
                None
            }
        };
        let ram = match self.parse_ram().await {
            Ok(info) => Some(info),
            Err(e) => {
                warn!("Failed to parse RAM: {e}");
                None
            }
        };

        OneRomLab { flash, ram }
    }

    async fn parse_flash(&mut self) -> Result<LabFlash, String> {
        let info = self.retrieve_flash_info().await?;

        let major_version = self
            .read_flash_string_at_ptr(info.major_version_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read major version: {e}"))
            .ok();
        let minor_version = self
            .read_flash_string_at_ptr(info.minor_version_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read minor version: {e}"))
            .ok();
        let patch_version = self
            .read_flash_string_at_ptr(info.patch_version_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read patch version: {e}"))
            .ok();
        let build_number = self
            .read_flash_string_at_ptr(info.build_number_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read build number: {e}"))
            .ok();
        let mcu = self
            .read_flash_string_at_ptr(info.mcu_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read MCU: {e}"))
            .ok();
        let hw_rev = self
            .read_flash_string_at_ptr(info.hw_rev_ptr)
            .await
            .inspect_err(|e| warn!("Failed to read HW revision: {e}"))
            .ok();

        let flash = LabFlash {
            major_version,
            minor_version,
            patch_version,
            build_number,
            mcu,
            hw_rev,
            rtt_ptr: info.rtt_ptr,
        };

        Ok(flash)
    }

    async fn parse_ram(&mut self) -> Result<LabRam, String> {
        let _info = self.retrieve_ram_info().await?;

        let ram = LabRam {};

        Ok(ram)
    }

    async fn read_flash_string_at_ptr(&mut self, ptr: u32) -> Result<String, String> {
        if ptr < self.base_flash_address {
            return Err(format!("Invalid pointer: 0x{:08X}", ptr));
        }

        read_string_at_ptr(&mut self.reader, ptr).await
    }
}
