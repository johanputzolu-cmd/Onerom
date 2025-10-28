// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Contains device's USB device handling

use dfu_rs::{DEFAULT_USB_TIMEOUT, DeviceInfo as DfuDeviceInfo, Device as DfuDevice, DfuType, search_for_dfu};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::time::Duration;

use crate::app::AppMessage;
use crate::device::{Client, Message};
use crate::hw::HardwareInfo;

/// Retrieve the list of connected USB devices.  Sends
/// Message::UsbDevicesDetected when done.
pub async fn get_usb_device_list_async() -> AppMessage {
    match search_for_dfu(DEFAULT_USB_TIMEOUT, Some(DfuType::InternalFlash)).await {
        Ok(devices) => {
            // Turn into UsbDeviceType
            let usb_devices: Vec<UsbDeviceType> = devices.into_iter().map(UsbDeviceType::from_dfu).filter_map(|d| d).collect();
            Message::UsbDevicesDetected(usb_devices).into()
        }
        Err(e) => {
            warn!("Failed to detect USB devices:\n  - {}", e);
            Message::UsbDevicesDetected(Vec::new()).into()
        }
    }
}

/// Retrieve the list of connected USB devices after a delay.  Used to give
/// time for the OS to enumerate devices after a reset.
pub async fn get_usb_device_list_delay(duration: Duration) -> AppMessage {
    tokio::time::sleep(duration).await;
    get_usb_device_list_async().await
}

/// A USB device type
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum UsbDeviceType {
    /// An STM32 bootloader
    Ice(DfuDevice),
    /// An RP2350 bootloader
    Fire(DfuDevice),
}

impl std::fmt::Display for UsbDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsbDeviceType::Ice(d) => write!(f, "Ice USB ({})", d.info()),
            UsbDeviceType::Fire(d) => write!(f, "Fire USB ({})", d.info()),
        }
    }
}

impl UsbDeviceType {
    pub fn dfu_device(&self) -> &DfuDevice {
        match self {
            UsbDeviceType::Ice(d) => d,
            UsbDeviceType::Fire(d) => d,
        }
    }

    pub fn dfu_device_info(&self) -> &DfuDeviceInfo {
        match self {
            UsbDeviceType::Ice(d) => d.info(),
            UsbDeviceType::Fire(d) => d.info(),
        }
    }

    pub fn from_dfu(dfu_device: DfuDevice) -> Option<Self> {
        match (dfu_device.info().vid, dfu_device.info().pid) {
            (0x0483, 0xDF11) => Some(UsbDeviceType::Ice(dfu_device)),
            (0x2E8A, 0x0005) => Some(UsbDeviceType::Fire(dfu_device)),
            _ => None,
        }
    }
}

/// Read memory from a device using USB DFU
pub async fn read_async(
    usb_device: UsbDeviceType,
    client: Client,
    _hw_info: HardwareInfo,
    address: u32,
    words: usize,
) -> AppMessage {
    match usb_device.dfu_device().upload(address, words*4).await {
        Ok(data) => Message::DeviceData(client, data).into(),
        Err(e) => {
            let log = format!("Failed to read {} words of memory at {address:#010X} using USB device {usb_device}: {e}", words);
            warn!("{log}");
            return Message::ReadFailed(client, log).into()
        }
    }
}

/// Flash firmware to a device using USB DFU
pub async fn flash_async(
    usb_device: UsbDeviceType,
    _hw_info: HardwareInfo,
    client: Client, 
    data: Vec<u8>,
) -> AppMessage {
    // Run the blocking USB operation on a separate thread
    debug!("Erase One ROM USB");
    match usb_device.dfu_device().mass_erase().await {
        Ok(()) => (),
        Err(e) => {
            let log = format!("Failed to mass erase One ROM using USB device {usb_device}: {e}");
            warn!("{log}");
            return Message::FlashFirmwareResult(client, Err(log)).into()
        }
    }
    debug!("Flash firmware to One ROM USB");
    match usb_device.dfu_device().download(0x08000000, &data).await {
        Ok(()) => {
            debug!("Successfully flashed firmware using USB device {usb_device}");
            Message::FlashFirmwareResult(client, Ok(())).into()
        },
        Err(e) => {
            let log = format!("Failed to flash firmware to One ROM using USB device {usb_device}: {e}");
            warn!("{log}");
            return Message::FlashFirmwareResult(client, Err(log)).into()
        }
    }
}   

