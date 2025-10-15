// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use futures_timer::Delay;
use iced::Element;
use iced::Task;
use iced::widget::row;
use probe_rs::MemoryInterface;
use probe_rs::Permissions;
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use std::time::Duration;

use crate::analyse::Message as AnalyseMessage;
use crate::app::Message as AppMessage;
use crate::style::Style;

const PROBE_DETECTION_RETRY_SECS: Duration = Duration::from_secs(10);

/// Messages for devices
#[derive(Debug, Clone)]
pub enum Message {
    DetectProbe(bool),
    ProbesDetected(Vec<DebugProbeInfo>),
    SelectProbe(DebugProbeInfo),
    ReadDevice {
        chip_id: String,
        address: u32,
        words: usize,
    },
}

/// Device state
#[derive(Debug, Clone)]
pub struct Device {
    selected_probe: Option<DebugProbeInfo>,
    probes: Option<Vec<DebugProbeInfo>>,
}

impl Device {
    pub fn new() -> Self {
        Self {
            selected_probe: None,
            probes: None,
        }
    }

    pub fn message(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::DetectProbe(delay) => self.get_probe_list(delay),
            Message::ProbesDetected(probes) => {
                self.probes = Some(probes);
                Task::none()
            }
            Message::SelectProbe(probe) => {
                self.selected_probe = Some(probe);
                Task::none()
            }
            Message::ReadDevice {
                chip_id,
                address,
                words,
            } => self.read_device(&chip_id, address, words),
        }
    }

    fn get_probe_list(&self, delay: bool) -> Task<AppMessage> {
        Task::perform(
            async move {
                if delay {
                    Delay::new(PROBE_DETECTION_RETRY_SECS).await;
                }
                let probes = Lister::new().list_all();
                if !probes.is_empty() {
                    AppMessage::Device(Message::ProbesDetected(probes))
                } else {
                    AppMessage::Device(Message::DetectProbe(true))
                }
            },
            |msg| msg,
        )
    }

    pub fn probe_pick_list(&self) -> Element<'_, AppMessage> {
        let list: Element<_> = if let Some(probes) = self.probes.as_ref() {
            let options = probes.as_slice();
            Style::pick_list(options, self.selected_probe.clone(), |p| {
                AppMessage::Device(Message::SelectProbe(p))
            })
            .into()
        } else {
            Style::text_h3("Not detected")
                .color(Style::COLOUR_DARK_GOLD)
                .into()
        };
        row![Style::text_h3("Probe:"), list,]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    }

    pub fn read_device(&self, chip_id: &str, address: u32, words: usize) -> Task<AppMessage> {
        let Some(probe_info) = self.selected_probe.as_ref() else {
            return Task::none();
        };

        let probe_info = probe_info.clone();
        let chip_id = chip_id.to_string();

        Task::perform(
            async move {
                match probe_info.open() {
                    Ok(probe) => match probe.attach(&chip_id, Permissions::default()) {
                        Ok(mut session) => match session.core(0) {
                            Ok(mut core) => {
                                let _ = core.halt(std::time::Duration::from_millis(100));

                                let mut buf = vec![0u32; words];
                                match core.read_32(address as u64, &mut buf) {
                                    Ok(()) => {
                                        let bytes: Vec<u8> =
                                            buf.iter().flat_map(|w| w.to_le_bytes()).collect();
                                        Some(bytes)
                                    }
                                    Err(e) => {
                                        eprintln!("Read failed: {}", e);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to get core: {}", e);
                                None
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to attach: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to open probe: {}", e);
                        None
                    }
                }
            },
            |opt| opt,
        )
        .then(|opt| match opt {
            Some(data) => Task::done(AppMessage::Analyse(AnalyseMessage::DeviceData(data))),
            None => Task::none(),
        })
    }
}
