// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use dfu_rs::{DeviceInfo as DfuDeviceInfo, Device as DfuDevice, DfuType, Error as DfuError};
use iced::widget::{column, row};
use iced::{Element, Subscription, Task, time};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use probe_rs::{MemoryInterface, Permissions, Core, Error as ProbeError};
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use std::time::Duration;

use crate::analyse::Message as AnalyseMessage;
use crate::app::AppMessage;
use crate::create::Message as CreateMessage;
use crate::studio::RuntimeInfo;
use crate::style::Style;

const DEVICE_DETECTION_RETRY_SHORT: Duration = Duration::from_secs(5);
const DEVICE_DETECTION_RETRY_LONG: Duration = Duration::from_secs(30);
const PROBE_CORE_HALT_TIMEOUT: Duration = Duration::from_millis(100);

/// A wrapper for DebugProbeInfo for use in pick_lists.
/// We do this so we can use the probe_type() not the default DebugProbeInfo
/// Display impl in the pick list
#[derive(Debug, Clone, PartialEq)]
struct DebugProbeInfoWrapper(DebugProbeInfo);

impl std::fmt::Display for DebugProbeInfoWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Your custom display logic here
        write!(f, "{} ({:04X}:{:04X})", self.0.probe_type(), self.0.vendor_id, self.0.product_id)
    }
}

impl Into<DebugProbeInfoWrapper> for DebugProbeInfo {
    fn into(self) -> DebugProbeInfoWrapper {
        DebugProbeInfoWrapper(self)
    }
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
    fn dfu_device(&self) -> &DfuDevice {
        match self {
            UsbDeviceType::Ice(d) => d,
            UsbDeviceType::Fire(d) => d,
        }
    }

    fn dfu_device_info(&self) -> &DfuDeviceInfo {
        match self {
            UsbDeviceType::Ice(d) => d.info(),
            UsbDeviceType::Fire(d) => d.info(),
        }
    }

    fn from_dfu(dfu_device: DfuDevice) -> Option<Self> {
        match (dfu_device.info().vid, dfu_device.info().pid) {
            (0x0483, 0xDF11) => Some(UsbDeviceType::Ice(dfu_device)),
            (0x2E8A, 0x0005) => Some(UsbDeviceType::Fire(dfu_device)),
            _ => None,
        }
    }
}

/// Messages for devices
#[derive(Debug, Clone)]
pub enum Message {
    DetectProbes,
    ProbesDetected(Vec<DebugProbeInfo>),
    SelectProbe(DebugProbeInfo),
    SelectUsbDevice(UsbDeviceType),
    SelectDevice(DeviceType),
    ReadDevice {
        chip_id: String,
        address: u32,
        words: usize,
    },
    DetectUsbDevices,
    UsbDevicesDetected(Vec<UsbDeviceType>),
    FlashFirmware(Vec<u8>),
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::DetectProbes => write!(f, "DetectProbes"),
            Message::ProbesDetected(probes) => {
                let probes_str = probes
                    .iter()
                    .map(|p| p.identifier.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "ProbesDetected({probes_str})")
            }
            Message::SelectDevice(device) => write!(f, "SelectDevice({})", device),
            Message::SelectProbe(probe) => write!(f, "SelectProbe({})", probe),
            Message::SelectUsbDevice(usb_device) => write!(f, "SelectUsbDevice({})", usb_device),
            Message::ReadDevice {
                chip_id,
                address,
                words,
            } => {
                write!(
                    f,
                    "ReadDevice(chip_id={}, address=0x{:X}, words={})",
                    chip_id, address, words
                )
            }
            Message::DetectUsbDevices => write!(f, "DetectUsbDevices"),
            Message::UsbDevicesDetected(devices) => {
                let devices_str = devices.iter()
                    .map(|d| format!("VID={:04X}, PID={:04X}", d.dfu_device_info().vid, d.dfu_device_info().pid))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "UsbDevicesDetected({})", devices_str)
            }
            Message::FlashFirmware(data) => {
                write!(f, "FlashFirmware({})", data.len())
            }
        }
    }
}

/// Device state
#[derive(Debug, Clone)]
pub struct Device {
    selected: DeviceType,
    selected_probe: Option<DebugProbeInfo>,
    selected_usb_device: Option<UsbDeviceType>,
    probes: Vec<DebugProbeInfo>,
    usb_devices: Vec<UsbDeviceType>,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            selected: DeviceType::None,
            selected_probe: None,
            selected_usb_device: None,
            probes: Vec::new(),
            usb_devices: Vec::new(),
        }
    }
}

impl Device {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected(&self) -> &DeviceType {
        &self.selected
    }

    pub fn update(&mut self, _runtime_info: &RuntimeInfo, message: Message) -> Task<AppMessage> {
        match message {
            Message::DetectProbes => Task::future(Self::get_probe_list_async()),
            Message::ProbesDetected(probes) => {
                self.probes_detected(probes);
                Task::none()
            }
            Message::SelectDevice(device) => self.select_device(device),
            Message::SelectProbe(probe) => self.select_probe(probe),
            Message::SelectUsbDevice(usb_device) => self.select_usb_device(usb_device),
            Message::ReadDevice {
                chip_id,
                address,
                words,
            } => self.selected.read(&chip_id, address, words),
            Message::DetectUsbDevices => Task::future(Self::get_usb_device_list_async()),
            Message::UsbDevicesDetected(devices) => {
                self.usb_devices_detected(devices);
                Task::none()
            }
            Message::FlashFirmware(data) => {
                self.selected.flash(data)
            }
        }
    }

    fn has_detected_probes(&self) -> bool {
        !self.probes.is_empty()
    }

    fn has_detected_usb_devices(&self) -> bool {
        !self.usb_devices.is_empty()
    }

    fn probes_detected(&mut self, probes: Vec<DebugProbeInfo>) {
        self.probes = probes.clone();
        if self.selected_probe.is_none() {
            self.selected_probe = probes.first().cloned();
            if self.selected_probe.is_some() {
                info!(
                    "Auto-selected probe: {}, {}",
                    self.selected_probe.as_ref().unwrap().identifier,
                    self.selected_probe.as_ref().unwrap().serial_number.as_deref().unwrap_or("N/A")
                );
            } else {
                trace!("No probes detected");
            }
        } else {
            // See if the globally selected device is the Probe
            let global_sel = if self.selected.debug_probe().is_some() {
                true
            } else {
                false
            };

            // Check if selected probe is still connected
            let still_connected = self.probes.iter().any(|p| {
                if let Some(selected_probe) = &self.selected_probe {
                    *p == *selected_probe
                } else {
                    false
                }
            });
            if !still_connected {
                let was_selected = self.selected_probe.clone().unwrap();
                self.selected_probe = None;
                info!(
                    "Selected probe has been disconnected: {}, {}",
                    was_selected.identifier,
                    was_selected.serial_number.as_deref().unwrap_or("N/A")
                );

                if global_sel {
                    self.selected = DeviceType::None;
                }
            }
        }

        // Finally, if there's no selected device, but there's a selected probe
        // device, select it
        if self.selected.is_none() {
            if let Some(probe) = &self.selected_probe {
                self.selected = DeviceType::from_debug_probe(probe.clone());
                info!("Auto-selected probe: {}, {}",
                    probe.identifier,
                    probe.serial_number.as_deref().unwrap_or("N/A"));
            }
        }
    }

    fn usb_devices_detected(&mut self, devices: Vec<UsbDeviceType>) {
        self.usb_devices = devices;

        if self.selected_usb_device.is_none() {
            self.selected_usb_device = self.usb_devices.first().cloned();
            if self.selected_usb_device.is_some() {
                info!(
                    "Auto-selected USB device: {}",
                    self.selected_usb_device.as_ref().unwrap()
                );
            } else {
                trace!("No USB devices detected");
            }
        } else {
            // If the globally selected device is USB
            let global_sel = if self.selected.usb_device().is_some() {
                true
            } else {
                false
            };

            // Check if selected USB device is still connected
            let still_connected = self.usb_devices.iter().any(|d| {
                if let Some(selected_usb) = &self.selected_usb_device {
                    *d == *selected_usb
                } else {
                    false
                }
            });
            if !still_connected {
                if let Some(was_selected) = &self.selected_usb_device {
                    info!(
                        "Selected USB device has been disconnected: {}",
                        was_selected
                    );
                }
                self.selected_usb_device = None;

                if global_sel {
                    self.selected = DeviceType::None;
                }
            }
        }

        // Finally, if there's no selected device, but there's a selected USB
        // device, select it
        if self.selected.is_none() {
            if let Some(usb_device) = &self.selected_usb_device {
                self.selected = DeviceType::from_usb(usb_device.clone());
                info!("Auto-selected active device: {}", usb_device);
            }
        } 
    }

    fn select_device(&mut self, device: DeviceType) -> Task<AppMessage> {
        self.selected = device;
        Task::none()
    }

    fn select_probe(&mut self, probe: DebugProbeInfo) -> Task<AppMessage> {
        self.selected_probe = Some(probe);
        Task::none()
    }

    fn select_usb_device(&mut self, usb_device: UsbDeviceType) -> Task<AppMessage> {
        self.selected_usb_device = Some(usb_device);
        Task::none()
    }

    async fn get_probe_list_async() -> AppMessage {
        let probes = Lister::new().list_all();
        if !probes.is_empty() {
            // Need to send ourselves a message, as we can't modify
            // self in this async block
            Message::ProbesDetected(probes).into()
        } else {
            Message::ProbesDetected(Vec::new()).into()
        }
    }

    async fn get_usb_device_list_async() -> AppMessage {
        match DfuDevice::search(Some(DfuType::InternalFlash)) {
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

    pub fn view(&self) -> Element<'_, AppMessage> {
        // Create the Probe and USB pick list labels
        let left_col = column![
            Style::text_small("Probe:"),
            Style::text_small("USB:"),
        ].spacing(20)
            .align_x(iced::alignment::Horizontal::Right);

        // Create the Probe pick list
        let probe_list: Element<'_, AppMessage> = if self.has_detected_probes() {
            let options = self.probes.clone().into_iter().map(DebugProbeInfoWrapper).collect::<Vec<_>>();
            Style::pick_list_small(options, self.selected_probe.clone().map(DebugProbeInfoWrapper), |p| {
                DeviceType::from_debug_probe(p.0.clone()).selected_message()
            })
            .into()
        } else {
            Style::text_body("Not detected")
                .color(Style::COLOUR_DARK_GOLD)
                .into()
        };

        // Create the USB device pick list
        let usb_device_list: Element<'_, AppMessage> = if !self.usb_devices.is_empty() {
            let options = self.usb_devices.as_slice();
            Style::pick_list_small(options, self.selected_usb_device.clone(), |d| {
                DeviceType::from_usb(d.clone()).selected_message()
            })
            .into()
        } else {
            Style::text_body("Not detected")
                .color(Style::COLOUR_DARK_GOLD)
                .into()
        };

        // Put the pick lists together into a column
        let right_col = column![
            probe_list,
            usb_device_list,
        ].spacing(10);

        // Create the row for the pick list and labels
        let pick_list_row = row![left_col, right_col]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center);

        // Figure out how the Probe/USB buttons should work
        let highlight_probe_button = self.selected().debug_probe().is_some();
        let highlight_usb_button = self.selected().usb_device().is_some();
        let on_press_probe = if self.selected().debug_probe().is_none() && self.selected_probe.is_some() {
            Some(Message::SelectDevice(
                DeviceType::from_debug_probe(self.selected_probe.as_ref().unwrap().clone()),
            ).into())
        } else {
            None
        };
        let on_press_usb = if self.selected().usb_device().is_none() && self.selected_usb_device.is_some() {
            Some(Message::SelectDevice(
                DeviceType::from_usb(self.selected_usb_device.as_ref().unwrap().clone()),
            ).into())
        } else {
            None
        };

        // Create the buttons
        let probe_button = Style::text_button_small("Probe", on_press_probe, highlight_probe_button);
        let usb_button = Style::text_button_small("USB", on_press_usb, highlight_usb_button);
        let button_row = row![
            Style::text_small("Device:"),
            probe_button,
            usb_button,
        ].spacing(20)
            .align_y(iced::alignment::Vertical::Center);

        column![
            button_row,
            pick_list_row,
        ]
            .spacing(20)
            .align_x(iced::alignment::Horizontal::Center)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let check_probes_duration = if self.has_detected_probes() {
            DEVICE_DETECTION_RETRY_LONG
        } else {
            DEVICE_DETECTION_RETRY_SHORT
        };
        let check_usb_devices_duration = if self.has_detected_usb_devices() {
            DEVICE_DETECTION_RETRY_LONG
        } else {
            DEVICE_DETECTION_RETRY_SHORT
        };
        
        Subscription::batch([
            time::every(check_probes_duration).map(|_| Message::DetectProbes),
            time::every(check_usb_devices_duration).map(|_| Message::DetectUsbDevices),
        ])
    }
}

/// A type of a device
#[derive(Debug, Default, Clone, PartialEq)]
pub enum DeviceType {
    /// None
    #[default]
    None,
    /// A device connected via a debug probe
    DebugProbe(DebugProbeInfo),
    /// A device connected via USB
    Usb(UsbDeviceType),
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::DebugProbe(info) => write!(
                f,
                "{}, {}",
                info.identifier,
                info.serial_number.as_deref().unwrap_or("N/A")
            ),
            DeviceType::Usb(usb_type) => write!(f, "Usb({})", usb_type),
            DeviceType::None => write!(f, "None"),
        }
    }
}

impl DeviceType {
    fn debug_probe(&self) -> Option<DebugProbeInfo> {
        if let DeviceType::DebugProbe(info) = self {
            Some(info.clone())
        } else {
            None
        }
    }

    fn usb_device(&self) -> Option<UsbDeviceType> {
        if let DeviceType::Usb(usb_type) = self {
            Some(usb_type.clone())
        } else {
            None
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, DeviceType::None)
    }

    fn from_debug_probe(info: DebugProbeInfo) -> Self {
        DeviceType::DebugProbe(info)
    }

    fn from_usb(usb_type: UsbDeviceType) -> Self {
        DeviceType::Usb(usb_type)
    }

    fn selected_message(&self) -> AppMessage {
        match self {
            DeviceType::DebugProbe(info) => Message::SelectProbe(info.clone()).into(),
            DeviceType::Usb(usb_type) => Message::SelectUsbDevice(usb_type.clone()).into(),
            DeviceType::None => unreachable!(),
        }
    }

    pub fn read(&self, chip_id: &str, address: u32, words: usize) -> Task<AppMessage> {
        match self {
            DeviceType::DebugProbe(probe) => Task::future(Self::read_debug_probe_async(
                probe.clone(),
                chip_id.to_string(),
                address,
                words,
            )),
            DeviceType::Usb(d) => Task::future(Self::read_usb_device_async(
                d.clone(),
                address,
                words,
            )),
            DeviceType::None => {
                error!(
                    "Internal error - attempted to read from None device - please raise a bug report"
                );
                Task::done(AnalyseMessage::ReadFailed(
                    "Internal error - attempted to read from None device - please raise a bug report".to_string(),
                ).into())
            }
        }
    }

    pub fn flash(&self, data: Vec<u8>) -> Task<AppMessage> {
        match self {
            DeviceType::DebugProbe(probe) => Task::future(Self::flash_firmware_probe_async(
                probe.clone(),
                data,
            )),
            DeviceType::Usb(usb_device) => Task::future(Self::flash_firmware_usb_async(
                usb_device.clone(),
                data,
            )),
            DeviceType::None => {
                error!(
                    "Internal error - attempted to flash to None device - please raise a bug report"
                );
                Task::done(CreateMessage::FlashFirmwareResult(Err(
                    "Internal error - attempted to flash to None device - please raise a bug report".to_string(),
                )).into())
            }
        }
    }

    pub async fn read_usb_device_async(
        usb_device: UsbDeviceType,
        address: u32,
        words: usize,
    ) -> AppMessage {
        // Allocate buffer for the read
        let mut buf = vec![0u32; words];
        
        let dfu_device = usb_device.dfu_device().clone();
        
        // Run the blocking USB operation on a separate thread
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<u32>, DfuError> {
            dfu_device.upload(address, &mut buf)?;
            Ok(buf)
        }).await;
        
        match result {
            Ok(Ok(data)) => {
                let bytes: Vec<u8> = data.iter().flat_map(|w| w.to_le_bytes()).collect();
                AnalyseMessage::DeviceData(bytes).into()
            }
            Ok(Err(e)) => {
                AnalyseMessage::ReadFailed(format!("DFU upload failed:\n  - {}", e)).into()
            }
            Err(e) => {
                AnalyseMessage::ReadFailed(format!("Task join failed:\n  - {}", e)).into()
            }
        }
    }

    pub async fn flash_firmware_usb_async(
        usb_device: UsbDeviceType,
        data: Vec<u8>,
    ) -> AppMessage {
        let dfu_device = usb_device.dfu_device().clone();

        // Convert vec<u8> to vec<u32>
        let data: Vec<u32> = data.chunks(4).map(|chunk| {
            let mut bytes = [0u8; 4];
            for (i, &b) in chunk.iter().enumerate() {
                bytes[i] = b;
            }
            u32::from_le_bytes(bytes)
        }).collect();

        // Run the blocking USB operation on a separate thread
        let result = tokio::task::spawn_blocking(move || -> Result<(), DfuError> {
            dfu_device.mass_erase()?;
            dfu_device.download(0x08000000, &data)?;
            Ok(())
        }).await;

        match result {
            Ok(Ok(())) => {
                debug!("Successfully flashed firmware using USB device {usb_device}");
                CreateMessage::FlashFirmwareResult(Ok(())).into()
            }
            Ok(Err(e)) => {
                warn!(
                    "Failed to flash firmware to One ROM using USB device {usb_device}: {e}",
                );
                CreateMessage::FlashFirmwareResult(Err(format!(
                    "Failed to flash firmware to One ROM using USB device {usb_device}:\n  - {e}",
                ))).into()
            }
            Err(e) => {
                warn!(
                    "Failed to flash firmware to One ROM using USB device {usb_device}: {e}",
                );
                CreateMessage::FlashFirmwareResult(Err(format!(
                    "Failed to flash firmware to One ROM using USB device {usb_device}:\n  - {e}",
                ))).into()
            }
        }
    }   

    async fn read_debug_probe_async(
        probe: DebugProbeInfo,
        chip_id: String,
        address: u32,
        words: usize,
    ) -> AppMessage {
        match Self::probe_init_and_operate(probe, chip_id, true, |core| {
            let mut buf = vec![0u32; words];
            core.read_32(address as u64, &mut buf)?;
            let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).collect();
            Ok(bytes)
        }).await {
            Ok(bytes) => AnalyseMessage::DeviceData(bytes).into(),
            Err(e) => {
                warn!("Failed to read {words} words of memory at {address:#010X}: {e}");
                AnalyseMessage::ReadFailed(format!(
                    "Failed to read {words} words of memory at {address:#010X}:\n  - {e}"
                )).into()
            }
        }
    }

    async fn flash_firmware_probe_async(
        probe: DebugProbeInfo,
        data: Vec<u8>,
    ) -> AppMessage {
        match Self::probe_init_and_operate(probe, "STM32F411RETx".to_string(), true, move |core| {
            core.write_8(0x08000000, &data)?;
            debug!("Successfully flashed firmware");
            Ok(())
        }).await {
            Ok(()) => CreateMessage::FlashFirmwareResult(Ok(())).into(),
            Err(e) => {
                warn!("Failed to flash firmware: {e}");
                CreateMessage::FlashFirmwareResult(Err(format!(
                    "Failed to flash firmware:\n  - {e}"
                ))).into()
            }
        }
    }

    // Helper to open a probe, attach to a chip, halt core, and run a closure
    async fn probe_init_and_operate<F, R>(
        probe: DebugProbeInfo, 
        chip_id: String, 
        halt_core: bool,
        f: F
    ) -> Result<R, ProbeError> 
    where
        F: FnOnce(&mut Core) -> Result<R, ProbeError>
    {
        let probe = probe.open()?;
        let probe_name = probe.get_name();
        let mut session = probe.attach(chip_id, Permissions::default())?;
        let mut core = session.core(0)?;

        if halt_core {
            debug!("Halting core using probe {}", probe_name);
            core.halt(PROBE_CORE_HALT_TIMEOUT)?;
        }

        f(&mut core)
    }

}
