// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Analyse image functionality

use iced::widget::{Space, column, row};
use iced::{Element, Length, Subscription, Task};
use rfd::FileDialog;
use std::path::PathBuf;

#[allow(unused_imports)]
use onerom_config::hw::{Board, MODELS, Model};
use onerom_config::mcu::Variant as McuVariant;
use sdrr_fw_parser::{Parser, SdrrInfo, readers::MemoryReader};

use crate::app::AppMessage;
use crate::device::Device;
use crate::hw::HardwareInfo;
use crate::studio::{Message as StudioMessage, RuntimeInfo};
use crate::style::Style;

/// Analyse tab messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    SourceTabSelected(SourceTab),
    DetectDevice,
    SelectFile,
    FileSelected(Option<PathBuf>),
    FileLoaded(Result<SdrrInfo, String>),
    DeviceData(Vec<u8>),
    ReadFailed(String),
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::SourceTabSelected(tab) => write!(f, "SourceTabSelected({:?})", tab),
            Message::DetectDevice => write!(f, "DetectDevice"),
            Message::SelectFile => write!(f, "SelectFile"),
            Message::FileSelected(_) => write!(f, "FileSelected(...)"),
            Message::FileLoaded(_) => write!(f, "FileLoaded(...)"),
            Message::DeviceData(_) => write!(f, "DeviceData(...)"),
            Message::ReadFailed(err) => write!(f, "ReadFailed({err})"),
        }
    }
}

/// Detect device state
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum DetectState {
    #[default]
    Ice,
    Fire,
    Done,
}

impl std::fmt::Display for DetectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectState::Ice => write!(f, "Ice"),
            DetectState::Fire => write!(f, "Fire"),
            DetectState::Done => write!(f, "Done"),
        }
    }
}

impl DetectState {
    pub fn next(&self) -> Self {
        match self {
            DetectState::Ice => DetectState::Fire,
            DetectState::Fire => DetectState::Done,
            DetectState::Done => DetectState::Done,
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, DetectState::Done)
    }

    /// We assume a specific STM32 MCU - doesn't matter which one as we're
    /// just readig common stuff, like flash base - and the chip ID will work
    /// for all.
    pub fn sample_mcu(&self) -> Option<McuVariant> {
        match self {
            DetectState::Ice => Some(McuVariant::F411RE),
            DetectState::Fire => Some(McuVariant::RP2350),
            DetectState::Done => None,
        }
    }

    pub fn flash_base(&self) -> Option<u32> {
        self.sample_mcu().map(|mcu| mcu.family().get_flash_base())
    }

    pub fn chip_id(&self) -> Option<String> {
        self.sample_mcu().map(|mcu| mcu.chip_id().to_string())
    }
}

/// Analyse tab state
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AnalyseState {
    #[default]
    Idle,
    Loading,
    Detecting(DetectState),
}

impl AnalyseState {
    #[allow(dead_code)]
    pub fn is_busy(&self) -> bool {
        !self.is_idle()
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, AnalyseState::Idle)
    }
}

impl std::fmt::Display for AnalyseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyseState::Idle => write!(f, "Idle"),
            AnalyseState::Loading => write!(f, "Loading"),
            AnalyseState::Detecting(state) => write!(f, "Detecting ({})", state),
        }
    }
}

impl AnalyseState {
    pub fn content(&self) -> String {
        match self {
            AnalyseState::Idle => Analyse::ANALYSIS_TEXT_DEFAULT.to_string(),
            AnalyseState::Loading => "Loading firmware...".to_string(),
            AnalyseState::Detecting(state) => format!("Trying to detect One ROM {state} ..."),
        }
    }
}

/// Analyse tab
#[derive(Debug, Clone)]
pub struct Analyse {
    selected_source_tab: SourceTab,
    analysis_content: String,
    fw_info: Option<SdrrInfo>,
    fw_file: Option<PathBuf>,
    state: AnalyseState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SourceTab {
    Device,
    #[default]
    File,
}

impl std::fmt::Display for SourceTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceTab::Device => write!(f, "Device"),
            SourceTab::File => write!(f, "File"),
        }
    }
}

impl Default for Analyse {
    fn default() -> Self {
        Self {
            analysis_content: Self::ANALYSIS_TEXT_DEFAULT.to_string(),
            selected_source_tab: Default::default(),
            fw_info: Default::default(),
            fw_file: Default::default(),
            state: Default::default(),
        }
    }
}

impl Analyse {
    // Button names
    const DEVICE_BUTTON_NAME: &'static str = "Device";
    const FILE_BUTTON_NAME: &'static str = "File";
    const SOURCE_DEVICE_BUTTON_NAME: &'static str = "Detect Device";
    const SOURCE_FILE_BUTTON_NAME: &'static str = "Select File";
    const ANALYSIS_TEXT_DEFAULT: &'static str = "No firmware analysed";

    pub const fn top_level_button_name() -> &'static str {
        "Analyse"
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, _runtime_info: &RuntimeInfo, message: Message) -> Task<AppMessage> {
        match message {
            Message::SourceTabSelected(tab) => {
                self.selected_source_tab = tab;
                Task::none()
            }
            Message::DetectDevice => {
                // Clear out previous analysis content
                self.analysis_content = String::new();
                self.detect_device(None)
            }
            Message::SelectFile => self.fw_file_chooser(),
            Message::FileSelected(path) => self.load_file(path),
            Message::FileLoaded(result) => self.file_loaded(result),
            Message::DeviceData(data) => Task::perform(
                async move {
                    // We always pass in 0x08000000 as the parser's base
                    // address even if RP2350 - parser will figure out what
                    // it's looking at
                    let mut reader = MemoryReader::new(data, 0x08000000);
                    let mut parser = Parser::new(&mut reader);
                    parser.parse_flash().await
                },
                |info| AppMessage::Analyse(Message::FileLoaded(info)),
            ),
            Message::ReadFailed(err) => {
                // Move onto trying to detect next device type
                self.detect_device(Some(err))
            }
        }
    }

    fn detect_device(&mut self, err: Option<String>) -> Task<AppMessage> {
        if let Some(err) = err {
            self.fw_info = None;
            self.analysis_content += &format!("\nError reading from device:\n- {err}\n");
        }

        // Move onto next detection state
        let new_state = match &self.state {
            AnalyseState::Detecting(state) => AnalyseState::Detecting(state.next()),
            _ => AnalyseState::Detecting(DetectState::default()),
        };
        let detect_state = match new_state.clone() {
            AnalyseState::Detecting(state) => state,
            _ => unreachable!(),
        };

        if detect_state.is_done() {
            self.fw_info = None;
            self.analysis_content += "---\nDevice detection failed - neither Ice nor Fire hardware detected.\nHave you connected the probe to the One ROM correctly, and does the One ROM have power?";
            self.state = AnalyseState::Idle;
            return Task::none();
        }

        // Actually do a detection, based on current state
        let start_analysis_task = self.start_analysis(new_state);
        let read_device_task = Task::done(AppMessage::Device(crate::device::Message::ReadDevice {
            chip_id: detect_state.chip_id().expect("Chip ID should be available"),
            address: detect_state
                .flash_base()
                .expect("Flash base should be available"),
            words: 65536 / 4,
        }));

        Task::chain(start_analysis_task, read_device_task)
    }

    fn file_loaded(&mut self, result: Result<SdrrInfo, String>) -> Task<AppMessage> {
        match result {
            Ok(info) => {
                let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string());
                self.analysis_content = match json {
                    Ok(j) => j,
                    Err(e) => format!("Error serializing info to JSON: {}", e),
                };
                self.fw_info = Some(info)
            }
            Err(err) => {
                self.fw_info = None;
                self.analysis_content = format!(
                    "Error loading/parsing file:\n- {}\n---\nAre you sure this is a valid One ROM firmware .bin file?",
                    err
                )
            }
        }
        self.state = AnalyseState::Idle;

        // Send decoded hardware informaton to the rest of the app
        self.share_hw_info()
    }

    fn share_hw_info(&mut self) -> Task<AppMessage> {
        if let Some(info) = self.fw_info.as_ref() {
            let hw_info = HardwareInfo {
                board: info.board,
                model: info.model,
                mcu_variant: info.mcu_variant,
            };
            Task::done(AppMessage::Studio(StudioMessage::HardwareInfo(Some(
                hw_info,
            ))))
        } else {
            Task::none()
        }
    }

    fn clear_hw_info(&self) -> Task<AppMessage> {
        Task::done(AppMessage::Studio(StudioMessage::HardwareInfo(None)))
    }

    fn start_analysis(&mut self, state: AnalyseState) -> Task<AppMessage> {
        self.state = state;
        self.analysis_content += &self.state.content().to_string();
        self.fw_info = None;
        self.clear_hw_info()
    }

    fn load_file(&mut self, path: Option<PathBuf>) -> Task<AppMessage> {
        if let Some(path) = path {
            let start_analysis_task = self.start_analysis(AnalyseState::Loading);
            self.fw_file = Some(path.clone());
            let load_file_task =
                Task::perform(async move { Self::async_load_file(path).await }, |info| {
                    AppMessage::Analyse(Message::FileLoaded(info))
                });
            Task::batch([start_analysis_task, load_file_task])
        } else {
            Task::none()
        }
    }

    async fn async_load_file(path: PathBuf) -> Result<SdrrInfo, String> {
        if path.exists() && path.is_file() {
            // Read in the file
            let data = std::fs::read(path).map_err(|e| e.to_string())?;

            // Parse it
            let mut reader = MemoryReader::new(data, 0x08000000);
            let mut parser = Parser::new(&mut reader);
            parser.parse_flash().await
        } else {
            Err("File does not exist or is a directory".to_string())
        }
    }

    fn fw_file_chooser(&self) -> Task<AppMessage> {
        Task::perform(
            async {
                FileDialog::new()
                    .add_filter("firmware", &["bin"])
                    .pick_file()
            },
            |path| AppMessage::Analyse(Message::FileSelected(path)),
        )
    }

    pub fn view(&self, runtime_info: &RuntimeInfo, device: &Device) -> Element<'_, AppMessage> {
        let hw_info = runtime_info.hw_info();

        let buttons = row![
            self.fw_source_buttons(),
            Space::with_width(Length::Fill),
            self.fw_source_control(device),
        ];

        column![
            column![
                self.select_fw_source(),
                buttons,
                Style::horiz_line(),
                self.fw_content_heading(hw_info),
            ]
            .spacing(20),
            Space::with_height(Length::Fixed(20.0)),
            Style::container(self.fw_content()),
        ]
        .into()
    }

    fn select_fw_source(&self) -> Element<'_, AppMessage> {
        row![Style::text_h3("Select Firmware Source")].into()
    }

    fn fw_source_buttons(&self) -> Element<'_, AppMessage> {
        // Determine button states based on selected tab
        let is_file_selected = matches!(self.selected_source_tab, SourceTab::File);

        let file_message = if is_file_selected {
            None
        } else {
            if self.state.is_idle() {
                Some(AppMessage::Analyse(Message::SourceTabSelected(
                    SourceTab::File,
                )))
            } else {
                None
            }
        };

        let device_message = if is_file_selected {
            if self.state.is_idle() {
                Some(AppMessage::Analyse(Message::SourceTabSelected(
                    SourceTab::Device,
                )))
            } else {
                None
            }
        } else {
            None
        };

        let file_button =
            Style::text_button(Self::FILE_BUTTON_NAME, file_message, is_file_selected);

        let device_button =
            Style::text_button(Self::DEVICE_BUTTON_NAME, device_message, !is_file_selected);

        row![file_button, device_button]
            .spacing(20)
            .padding(10)
            .into()
    }

    fn fw_source_control(&self, device: &Device) -> Element<'_, AppMessage> {
        match self.selected_source_tab {
            SourceTab::Device => self.fw_source_device_control(device),
            SourceTab::File => self.fw_source_file_control(),
        }
    }

    fn fw_source_device_control(&self, device: &Device) -> Element<'_, AppMessage> {
        let highlighted = if self.state.is_idle() && !device.selected().is_none() {
            true
        } else {
            false
        };

        let message = if self.state.is_idle() && !device.selected().is_none() {
            Some(AppMessage::Analyse(Message::DetectDevice))
        } else {
            None
        };

        let content = if self.state.is_idle() {
            Self::SOURCE_DEVICE_BUTTON_NAME
        } else {
            "Detecting..."
        };

        let button = Style::text_button(content, message, highlighted);
        row![button].spacing(20).padding(10).into()
    }

    fn fw_source_file_control(&self) -> Element<'_, AppMessage> {
        // Only enable this button if file not being loaded
        let file_control_message = if self.state.is_idle() {
            Some(AppMessage::Analyse(Message::SelectFile))
        } else {
            None
        };

        let content = if self.state.is_idle() {
            Self::SOURCE_FILE_BUTTON_NAME
        } else {
            "Loading..."
        };

        // Create the button
        let button = Style::text_button(content, file_control_message, true);

        // Create the row
        row![button].spacing(20).padding(10).into()
    }

    fn fw_content_heading(&self, hw_info: Option<&HardwareInfo>) -> Element<'_, AppMessage> {
        // Include hardware info if available
        let heading = Style::text_h3("Analysis");
        if let Some(hw_info) = hw_info {
            let version = self.fw_info.as_ref().and_then(|info| Some(info.version));
            let info_row = Style::hw_info_row(
                version,
                hw_info.model,
                hw_info.board,
                hw_info.mcu_variant,
                false,
            );

            row![heading, Space::with_width(Length::Fill), info_row,]
                .align_y(iced::alignment::Vertical::Center)
        } else {
            row![heading]
        }
        .into()
    }

    fn fw_content(&self) -> Element<'_, AppMessage> {
        Style::box_scrollable_text(&self.analysis_content, 320.0).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
