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
use sdrr_fw_parser::{Parser, SdrrInfo, readers::MemoryReader};

use crate::app::AppMessage;
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
        }
    }
}

/// Analyse tab state
#[derive(Debug, Clone)]
pub struct Analyse {
    selected_source_tab: SourceTab,
    analysis_content: String,
    fw_info: Option<SdrrInfo>,
    fw_file: Option<PathBuf>,
    loading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTab {
    Device,
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
        Self {
            selected_source_tab: SourceTab::File,
            analysis_content: Self::ANALYSIS_TEXT_DEFAULT.to_string(),
            fw_info: None,
            fw_file: None,
            loading: false,
        }
    }

    pub fn update(&mut self, _runtime_info: &RuntimeInfo, message: Message) -> Task<AppMessage> {
        match message {
            Message::SourceTabSelected(tab) => {
                self.selected_source_tab = tab;
                Task::none()
            }
            Message::DetectDevice => self.detect_device(),
            Message::SelectFile => self.fw_file_chooser(),
            Message::FileSelected(path) => self.load_file(path),
            Message::FileLoaded(result) => self.file_loaded(result),
            Message::DeviceData(data) => Task::perform(
                async move {
                    let mut reader = MemoryReader::new(data, 0x08000000);
                    let mut parser = Parser::new(&mut reader);
                    parser.parse_flash().await
                },
                |info| AppMessage::Analyse(Message::FileLoaded(info)),
            ),
        }
    }

    fn detect_device(&mut self) -> Task<AppMessage> {
        Task::done(AppMessage::Device(crate::device::Message::ReadDevice {
            chip_id: "STM32F411RETx".to_string(),
            address: 0x08000000,
            words: 65536 / 4,
        }))
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
                self.analysis_content = format!("Error loading/parsing file: {}", err)
            }
        }
        self.loading = false;

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
            Task::done(AppMessage::Studio(StudioMessage::HardwareInfo(hw_info)))
        } else {
            Task::none()
        }
    }

    fn load_file(&mut self, path: Option<PathBuf>) -> Task<AppMessage> {
        self.fw_info = None;
        self.analysis_content = "Loading...".to_string();
        self.loading = true;

        if let Some(path) = path {
            self.fw_file = Some(path.clone());
            Task::perform(async move { Self::async_load_file(path).await }, |info| {
                AppMessage::Analyse(Message::FileLoaded(info))
            })
        } else {
            self.fw_file = None;
            self.analysis_content = Self::ANALYSIS_TEXT_DEFAULT.to_string();
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

    pub fn view(&self, runtime_info: &RuntimeInfo) -> Element<'_, AppMessage> {
        let hw_info = runtime_info.hw_info();

        let buttons = row![
            self.fw_source_buttons(),
            Space::with_width(Length::Fill),
            self.fw_source_control(),
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
            Some(AppMessage::Analyse(Message::SourceTabSelected(SourceTab::File)))
        };
        
        let device_message = if is_file_selected {
            Some(AppMessage::Analyse(Message::SourceTabSelected(SourceTab::Device)))
        } else {
            None
        };

        let file_button = Style::text_button(
            Self::FILE_BUTTON_NAME,
            file_message,
            is_file_selected,
        );
        
        let device_button = Style::text_button(
            Self::DEVICE_BUTTON_NAME,
            device_message,
            !is_file_selected,
        );

        row![file_button, device_button]
            .spacing(20)
            .padding(10)
            .into()
    }

    fn fw_source_control(&self) -> Element<'_, AppMessage> {
        match self.selected_source_tab {
            SourceTab::Device => self.fw_source_device_control(),
            SourceTab::File => self.fw_source_file_control(),
        }
    }

    fn fw_source_device_control(&self) -> Element<'_, AppMessage> {
        let button = Style::text_button(
            Self::SOURCE_DEVICE_BUTTON_NAME,
            Some(AppMessage::Analyse(Message::DetectDevice)),
            true,
        );
        row![button].spacing(20).padding(10).into()
    }

    fn fw_source_file_control(&self) -> Element<'_, AppMessage> {
        // Only enable this button if file not being loaded
        let file_control_message = if self.loading {
            None
        } else {
            Some(AppMessage::Analyse(Message::SelectFile))
        };

        // Create the button
        let button = Style::text_button(Self::SOURCE_FILE_BUTTON_NAME, file_control_message, true);

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
