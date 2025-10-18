// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use iced::widget::row;
use iced::{Element, Subscription, Task, time};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use probe_rs::MemoryInterface;
use probe_rs::Permissions;
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use std::time::Duration;

use crate::analyse::Message as AnalyseMessage;
use crate::app::AppMessage;
use crate::studio::RuntimeInfo;
use crate::style::Style;

const PROBE_DETECTION_RETRY_SHORT: Duration = Duration::from_secs(5);
const PROBE_DETECTION_RETRY_LONG: Duration = Duration::from_secs(30);

/// A USB device type
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum UsbDeviceType {
    /// An STM32 bootloader
    Stm32Bootloader,
    /// An RP2350 bootloader
    Rp2350Bootloader,
}

impl std::fmt::Display for UsbDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsbDeviceType::Stm32Bootloader => write!(f, "Ice USB"),
            UsbDeviceType::Rp2350Bootloader => write!(f, "Fire USB"),
        }
    }
}

/// Messages for devices
#[derive(Debug, Clone)]
pub enum Message {
    DetectProbe,
    ProbesDetected(Vec<DebugProbeInfo>),
    SelectDevice(DeviceType),
    ReadDevice {
        chip_id: String,
        address: u32,
        words: usize,
    },
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::DetectProbe => write!(f, "DetectProbe"),
            Message::ProbesDetected(probes) => {
                let probes_str = probes
                    .iter()
                    .map(|p| p.identifier.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "ProbesDetected({probes_str})")
            }
            Message::SelectDevice(device) => write!(f, "SelectDevice({})", device),
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
        }
    }
}

/// Device state
#[derive(Debug, Clone)]
pub struct Device {
    selected: DeviceType,
    probes: Vec<DebugProbeInfo>,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            selected: DeviceType::None,
            probes: Vec::new(),
        }
    }
}

impl Device {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, _runtime_info: &RuntimeInfo, message: Message) -> Task<AppMessage> {
        match message {
            Message::DetectProbe => Task::future(Self::get_probe_list_async()),
            Message::ProbesDetected(probes) => {
                self.probes_detected(probes);
                Task::none()
            }
            Message::SelectDevice(device) => self.select_device(device),
            Message::ReadDevice {
                chip_id,
                address,
                words,
            } => self.selected.read(&chip_id, address, words),
        }
    }

    fn has_detected_probes(&self) -> bool {
        !self.probes.is_empty()
    }

    fn probes_detected(&mut self, probes: Vec<DebugProbeInfo>) {
        self.probes = probes;
        match self.selected.clone() {
            DeviceType::None => {
                // No device selected, auto-select
                if let Some(first_probe) = self.probes.first() {
                    self.selected = DeviceType::from_debug_probe(first_probe.clone());
                    info!(
                        "Auto-selected probe: {}, {}",
                        first_probe.identifier,
                        first_probe.serial_number.as_deref().unwrap_or("N/A")
                    )
                } else {
                    trace!("No probes detected")
                }
            }
            DeviceType::DebugProbe(selected_probe) => {
                // Check if selected device is still connected
                let still_connected = self.probes.iter().any(|p| *p == selected_probe);
                if !still_connected {
                    let was_selected = self.selected.clone();
                    self.selected = DeviceType::None;
                    info!("Selected probe has been disconnected: {}", was_selected)
                } else {
                    trace!("Selected probe still connected: {}", self.selected)
                }
            }
            _ => {
                trace!("USB device selected, not checking probes")
            }
        }
    }

    fn select_device(&mut self, device: DeviceType) -> Task<AppMessage> {
        self.selected = device;
        Task::none()
    }

    async fn get_probe_list_async() -> AppMessage {
        let probes = Lister::new().list_all();
        if !probes.is_empty() {
            // Need to send ourselves a message, as we can't modify
            // self in this async block
            AppMessage::Device(Message::ProbesDetected(probes))
        } else {
            AppMessage::Device(Message::ProbesDetected(Vec::new()))
        }
    }

    pub fn probe_pick_list(&self) -> Element<'_, AppMessage> {
        let list: Element<_> = if self.has_detected_probes() {
            let options = self.probes.as_slice();
            Style::pick_list_small(options, self.selected.debug_probe(), |p| {
                DeviceType::from_debug_probe(p.clone()).selected_message()
            })
            .into()
        } else {
            Style::text_body("Not detected")
                .color(Style::COLOUR_DARK_GOLD)
                .into()
        };
        row![Style::text_body("Probe:"), list,]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let check_probes_duration = if self.has_detected_probes() {
            PROBE_DETECTION_RETRY_LONG
        } else {
            PROBE_DETECTION_RETRY_SHORT
        };
        time::every(check_probes_duration).map(|_| Message::DetectProbe)
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

    fn from_debug_probe(info: DebugProbeInfo) -> Self {
        DeviceType::DebugProbe(info)
    }

    fn from_usb(usb_type: UsbDeviceType) -> Self {
        DeviceType::Usb(usb_type)
    }

    fn from_none() -> Self {
        DeviceType::None
    }

    fn selected_message(&self) -> AppMessage {
        match self {
            DeviceType::DebugProbe(info) => AppMessage::Device(Message::SelectDevice(
                DeviceType::from_debug_probe(info.clone()),
            )),
            DeviceType::Usb(usb_type) => AppMessage::Device(Message::SelectDevice(
                DeviceType::from_usb(usb_type.clone()),
            )),
            DeviceType::None => AppMessage::Device(Message::SelectDevice(DeviceType::from_none())),
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
            DeviceType::Usb(_) => {
                // USB device reading not implemented yet
                Task::none()
            }
            DeviceType::None => Task::none(),
        }
    }

    pub async fn read_debug_probe_async(
        probe: DebugProbeInfo,
        chip_id: String,
        address: u32,
        words: usize,
    ) -> AppMessage {
        let probe = match probe.open() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to open probe: {}", e);
                return AnalyseMessage::ReadFailed(format!("Failed to open probe:\n  - {}", e))
                    .into();
            }
        };

        let mut session = match probe.attach(chip_id, Permissions::default()) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to attach to device: {}", e);
                return AnalyseMessage::ReadFailed(format!(
                    "Failed to attach to device:\n  - {}",
                    e
                ))
                .into();
            }
        };

        let mut core = match session.core(0) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to connect to MCU: {}", e);
                return AnalyseMessage::ReadFailed(format!(
                    "Failed to connect to device's MCU:\n  - {}",
                    e
                ))
                .into();
            }
        };

        let _ = core.halt(std::time::Duration::from_millis(100));

        let mut buf = vec![0u32; words];
        match core.read_32(address as u64, &mut buf) {
            Ok(()) => {
                let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).collect();
                AnalyseMessage::DeviceData(bytes).into()
            }
            Err(e) => {
                warn!("Memory read from device failed: {}", e);
                AnalyseMessage::ReadFailed(format!("Failed to read memory from device:\n  - {}", e))
                    .into()
            }
        }
    }
}
