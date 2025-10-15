// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Analyse image functionality

use iced::widget::{Space, column, row};
use iced::{Element, Length, Task};
use rfd::FileDialog;
use std::path::PathBuf;

#[allow(unused_imports)]
use onerom_config::hw::{Board, MODELS, Model};
use onerom_config::mcu::Variant as McuVariant;
use sdrr_fw_parser::{Parser, SdrrInfo, readers::MemoryReader};

use crate::app::{Message as AppMessage, StudioMessage};
use crate::style::Style;

/// Analyse tab messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    BoardSelected(Board),
    ModelSelected(Model),
    McuSelected(McuVariant),
    SourceTabSelected(SourceTab),
    DetectDevice,
    SelectFile,
    FileSelected(Option<PathBuf>),
    FileLoaded(Result<SdrrInfo, String>),
    HardwareInfo(HardwareInfo),
    DeviceData(Vec<u8>),
}

/// Analyse tab state
#[derive(Debug, Clone)]
pub struct Analyse {
    selected_model: Option<Model>,
    selected_board: Option<Board>,
    selected_mcu: Option<McuVariant>,
    selected_source_tab: SourceTab,
    analysis_content: String,
    fw_info: Option<SdrrInfo>,
    fw_file: Option<PathBuf>,
    loading: bool,
    hw_info: Option<HardwareInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTab {
    Device,
    File,
}

/// Information about hardware
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareInfo {
    pub board: Option<Board>,
    pub model: Option<Model>,
    pub mcu_variant: Option<McuVariant>,
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

    pub const fn heading() -> &'static str {
        "Analyse"
    }

    pub fn new() -> Self {
        Self {
            selected_model: None,
            selected_board: None,
            selected_mcu: None,
            selected_source_tab: SourceTab::File,
            analysis_content: Self::ANALYSIS_TEXT_DEFAULT.to_string(),
            fw_info: None,
            fw_file: None,
            loading: false,
            hw_info: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::BoardSelected(board) => {
                self.selected_board = Some(board);
                Task::none()
            }
            Message::ModelSelected(model) => {
                self.selected_board = None;
                self.selected_model = Some(model);
                Task::none()
            }
            Message::McuSelected(mcu) => {
                self.selected_mcu = Some(mcu);
                Task::none()
            }
            Message::SourceTabSelected(tab) => {
                self.selected_source_tab = tab;
                Task::none()
            }
            Message::DetectDevice => self.detect_device(),
            Message::SelectFile => self.fw_file_chooser(),
            Message::FileSelected(path) => self.load_file(path),
            Message::FileLoaded(result) => self.file_loaded(result),
            Message::HardwareInfo(hw_info) => {
                self.hw_info = Some(hw_info);
                Task::none()
            }
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
        self.hw_info = None;
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

    pub fn view(&self) -> Element<'_, AppMessage> {
        column![
            column![
                self.select_fw_source_heaing_row(),
                self.fw_source_buttons(),
                Style::horiz_line(),
                self.fw_source_control(),
                Style::horiz_line(),
                self.fw_content_heading(),
            ]
            .spacing(20),
            Space::with_height(Length::Fixed(20.0)),
            Style::container(self.fw_content()),
        ]
        .into()
    }

    fn select_fw_source_heaing_row(&self) -> Element<'_, AppMessage> {
        row![Style::text_h3("Select Firmware Source")].into()
    }

    fn fw_source_buttons(&self) -> Element<'_, AppMessage> {
        // Figure out which button is highlighted, and which performs an action if clicked
        let file_message = Some(AppMessage::Analyse(Message::SourceTabSelected(
            SourceTab::File,
        )));
        let device_message = Some(AppMessage::Analyse(Message::SourceTabSelected(
            SourceTab::Device,
        )));

        let ((file_message, device_message), (file_highlighted, device_highlighted)) =
            match self.selected_source_tab {
                SourceTab::Device => ((file_message, None), (false, true)),
                SourceTab::File => ((None, device_message), (true, false)),
            };

        // Create the buttons
        let device_button =
            Style::text_button(Self::DEVICE_BUTTON_NAME, device_message, device_highlighted);
        let file_button =
            Style::text_button(Self::FILE_BUTTON_NAME, file_message, file_highlighted);

        // Create the row
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

    fn fw_content_heading(&self) -> Element<'_, AppMessage> {
        // Include hardware info if available
        let heading = Style::text_h3("Analysis");
        if let Some(hw_info) = self.hw_info.as_ref() {
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
        Style::box_scrollable(&self.analysis_content, 225.0).into()
    }
}
