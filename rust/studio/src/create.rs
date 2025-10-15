// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Create functionality

use iced::Task;
use iced::widget::{column, row};

use onerom_config::hw::{Board, MODELS, Model};
use onerom_config::mcu::{Family, MCU_VARIANTS, Variant as McuVariant};
use onerom_fw::net::{Release, Releases};

use crate::analyse::HardwareInfo;
use crate::app::{Message as AppMessage, StudioMessage};
use crate::style::Style;

#[derive(Debug, Clone)]

/// Create tab messages
pub enum Message {
    BoardSelected(Board),
    ModelSelected(Model),
    McuSelected(McuVariant),
    DetectHardware,
    HardwareInfo(HardwareInfo),
    Releases(Releases),
    ReleaseSelected(Release),
    ReleaseDownloaded(Vec<u8>),
}

/// Create tab state
#[derive(Debug, Clone)]
pub struct Create {
    selected_model: Option<Model>,
    selected_board: Option<Board>,
    selected_mcu: Option<McuVariant>,
    mcu_variants: Option<Vec<McuVariant>>,
    hw_info: Option<HardwareInfo>,
    releases: Option<Releases>,
    selected_release: Option<Release>,
    downloaded_firmware: Option<Vec<u8>>,
}

impl Create {
    pub const fn top_level_button_name() -> &'static str {
        "Create"
    }

    pub const fn heading() -> &'static str {
        "Create"
    }

    pub fn new() -> Self {
        Self {
            selected_model: None,
            selected_board: None,
            selected_mcu: None,
            mcu_variants: None,
            hw_info: None,
            releases: None,
            selected_release: None,
            downloaded_firmware: None,
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<AppMessage> {
        match message {
            Message::ModelSelected(model) => {
                self.model_selected(model);
                Task::none()
            }
            Message::BoardSelected(board) => self.board_selected(board),
            Message::DetectHardware => Task::none(),
            Message::McuSelected(mcu) => self.mcu_selected(mcu),
            Message::HardwareInfo(hw_info) => {
                self.hw_info = Some(hw_info);
                if let Some(model) = hw_info.model {
                    self.model_selected(model);
                }
                let t1 = if let Some(board) = hw_info.board {
                    self.board_selected(board)
                } else {
                    Task::none()
                };
                let t2 = if let Some(mcu) = hw_info.mcu_variant {
                    self.mcu_selected(mcu)
                } else {
                    Task::none()
                };
                Task::batch([t1, t2])
            }
            Message::Releases(releases) => {
                self.releases = Some(releases);
                if self.hardware_selected() {
                    self.select_latest_release()
                } else {
                    Task::none()
                }
            }
            Message::ReleaseSelected(release) => self.release_selected(release),
            Message::ReleaseDownloaded(data) => {
                self.downloaded_firmware = Some(data);
                Task::none()
            }
        }
    }

    fn select_latest_release(&mut self) -> Task<AppMessage> {
        if let Some(releases) = &self.releases {
            let latest = releases.latest();
            let latest = releases.release_from_string(latest);
            if let Some(r) = latest {
                return self.release_selected(r.clone());
            }
        }
        Task::none()
    }

    fn release_selected(&mut self, release: Release) -> Task<AppMessage> {
        self.selected_release = Some(release.clone());
        self.downloaded_firmware = None;

        // Download the release
        if let Some(board) = self.selected_board
            && let Some(mcu) = self.selected_mcu
        {
            Task::done(AppMessage::Studio(StudioMessage::DownloadRelease(
                release, board, mcu,
            )))
        } else {
            eprintln!("Board or MCU not selected, cannot download firmware");
            Task::none()
        }
    }

    fn model_selected(&mut self, model: Model) {
        self.selected_model = Some(model);
        self.selected_board = None;
        self.selected_mcu = None;
        self.mcu_variants = None;
    }

    fn board_selected(&mut self, board: Board) -> Task<AppMessage> {
        self.selected_board = Some(board);
        let mut vars = Vec::new();
        for var in MCU_VARIANTS {
            if board.mcu_family() == var.family() {
                vars.push(*var);
            }
        }
        self.mcu_variants = Some(vars);

        // Special case the Fire boards
        if board.mcu_family() == Family::Rp2350 {
            self.mcu_selected(McuVariant::RP2350)
        } else {
            Task::none()
        }
    }

    fn mcu_selected(&mut self, mcu: McuVariant) -> Task<AppMessage> {
        self.selected_mcu = Some(mcu);

        // If we're ready, select the latest release
        if self.hardware_selected() {
            self.downloaded_firmware = None;
            self.selected_release = None;
            self.select_latest_release()
        } else {
            Task::none()
        }
    }

    fn hardware_selected(&self) -> bool {
        self.selected_model.is_some()
            && self.selected_board.is_some()
            && self.selected_mcu.is_some()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        let mut columns = column![
            row![
                self.select_hw_heading_row(),
                Style::text_h3("or"),
                self.detect_button(),
            ]
            .spacing(20)
            .align_y(iced::alignment::Vertical::Center),
            self.select_hw_row(),
            self.board_description_row(),
            Style::horiz_line()
        ];

        if self.hardware_selected() {
            // Add row to column
            columns = columns.push(self.firmware_row()).push(Style::horiz_line());
        }

        columns.spacing(20).into()
    }

    fn firmware_row(&self) -> iced::Element<'_, AppMessage> {
        // Create release selection row
        if let Some(releases) = &self.releases {
            let latest = releases.latest();

            let selected_release = if let Some(r) = &self.selected_release {
                Some(r)
            } else {
                releases.release_from_string(latest)
            };

            let mut rows = row![
                Style::text_h3("Select Firmware Release"),
                Style::pick_list(releases.releases().as_slice(), selected_release, |r| {
                    AppMessage::Create(Message::ReleaseSelected(r))
                })
            ];

            // Show if release has been downloaded
            if let Some(fw) = self.downloaded_firmware.as_ref() {
                // split into three rows, with number of bytes gold
                let downloaded_row = row![
                    Style::text_small("(downloaded: "),
                    Style::text_small(format!("{}", fw.len())).color(Style::COLOUR_DARK_GOLD),
                    Style::text_small(" bytes)"),
                ]
                .spacing(0);
                rows = rows.push(downloaded_row);
            }

            // Return the row
            rows.spacing(20).align_y(iced::alignment::Vertical::Center)
        } else {
            row![Style::text_h3("No firmware releases available")]
        }
        .spacing(20)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn select_hw_heading_row(&self) -> iced::Element<'_, AppMessage> {
        row![Style::text_h3("Select Hardware")].into()
    }

    fn detect_button(&self) -> iced::Element<'_, AppMessage> {
        let button = Style::text_button(
            "Detect Hardware",
            Some(Message::DetectHardware.into()),
            true,
        );
        row![button].into()
    }

    fn select_hw_row(&self) -> iced::Element<'_, AppMessage> {
        // Set up model picker
        let model_picker = Style::pick_list(MODELS.as_slice(), self.selected_model, |model| {
            AppMessage::Create(Message::ModelSelected(model))
        });

        // Set up board picker
        let board_values = if let Some(model) = self.selected_model {
            model.boards()
        } else {
            &[]
        };
        let board_picker = Style::pick_list(board_values, self.selected_board, |board| {
            AppMessage::Create(Message::BoardSelected(board))
        });

        // Set up MCU picker
        let mcu_values = if let Some(vars) = &self.mcu_variants {
            vars.as_slice()
        } else {
            &[]
        };
        let mcu_picker = Style::pick_list(mcu_values, self.selected_mcu, |mcu| {
            AppMessage::Create(Message::McuSelected(mcu))
        });

        row![model_picker, board_picker, mcu_picker]
            .spacing(20)
            .into()
    }

    fn board_description_row(&self) -> iced::Element<'_, AppMessage> {
        if self.hardware_selected() {
            let hw_info_row = Style::hw_info_row(
                None,
                self.selected_model,
                self.selected_board,
                self.selected_mcu,
                true,
            );

            row![
                Style::text_body("Selected:").color(Style::COLOUR_GOLD),
                hw_info_row,
            ]
            .spacing(20)
            .align_y(iced::alignment::Vertical::Center)
        } else {
            row![Style::text_body("Hardware not selected")]
        }
        .into()
    }
}
