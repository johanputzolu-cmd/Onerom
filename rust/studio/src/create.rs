// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Create functionality

use iced::widget::{column, row};
use iced::{Subscription, Task};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use onerom_config::hw::{Board, MODELS, Model};
use onerom_config::mcu::{Family, MCU_VARIANTS, Variant as McuVariant};
use onerom_fw::net::{Release, Releases};

use crate::app::AppMessage;
use crate::hw::HardwareInfo;
use crate::studio::{Message as StudioMessage, RuntimeInfo};
use crate::style::Style;
use crate::{task_from_msg, task_from_msgs};

#[derive(Debug, Clone)]
/// Create tab messages
pub enum Message {
    /// Board selection pick list value changed
    BoardSelected(Board),
    /// Model selection pick list value changed
    ModelSelected(Model),
    /// MCU selection pick list value changed
    McuSelected(McuVariant),
    /// Detect hardware button pressed
    DetectHardware,
    /// Firmware release selected via pick list
    ReleaseSelected(Release),
    /// Releases have been updated (from network)
    ReleasesUpdated,
    /// Hardware information detected from a device or firmware file
    DetectedHardwareInfo,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::BoardSelected(board) => write!(f, "BoardSelected({})", board.name()),
            Message::ModelSelected(model) => write!(f, "ModelSelected({})", model.name()),
            Message::McuSelected(mcu) => write!(f, "McuSelected({mcu})"),
            Message::DetectHardware => write!(f, "DetectHardware"),
            Message::ReleaseSelected(release) => {
                write!(f, "ReleaseSelected({})", release.version)
            }
            Message::ReleasesUpdated => write!(f, "ReleasesUpdated"),
            Message::DetectedHardwareInfo => write!(f, "DetectedHardwareInfo"),
        }
    }
}

/// Create tab state
#[derive(Debug, Default, Clone)]
pub struct Create {
    selected_hw_info: HardwareInfo,
    mcu_variants: Option<Vec<McuVariant>>,
}

impl Create {
    pub const fn top_level_button_name() -> &'static str {
        "Create"
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        runtime_info: &RuntimeInfo,
        message: Message,
    ) -> iced::Task<AppMessage> {
        match message {
            Message::ModelSelected(model) => {
                self.model_selected(model);
                Task::none()
            }
            Message::BoardSelected(board) => {
                task_from_msg!(self.board_selected(runtime_info, board))
            }
            Message::DetectHardware => Task::none(),
            Message::McuSelected(mcu) => {
                self.mcu_selected(mcu);
                task_from_msg!(self.select_latest_release(runtime_info.releases()))
            }
            Message::DetectedHardwareInfo => {
                if let Some(hw_info) = runtime_info.hw_info() {
                    if let Some(model) = hw_info.model {
                        self.model_selected(model);
                    }

                    let msg1 = if let Some(board) = hw_info.board {
                        self.board_selected(runtime_info, board)
                    } else {
                        None
                    };

                    let msg2 = if let Some(mcu) = hw_info.mcu_variant {
                        self.mcu_selected(mcu);
                        self.select_latest_release(runtime_info.releases())
                    } else {
                        None
                    };
                    task_from_msgs!([msg1, msg2])
                } else {
                    warn!("No hardware info available");
                    Task::none()
                }
            }
            Message::ReleasesUpdated => {
                let releases = runtime_info.releases();
                if self.hardware_selected() {
                    task_from_msg!(self.select_latest_release(releases))
                } else {
                    Task::none()
                }
            }
            Message::ReleaseSelected(release) => task_from_msg!(self.release_selected(release)),
        }
    }

    fn select_latest_release(&mut self, releases: Option<&Releases>) -> Option<AppMessage> {
        // Only select latest if hardware is fully selected
        if !self.hardware_selected() {
            return None;
        }

        if let Some(releases) = releases {
            let latest = releases.latest();
            let latest = releases.release_from_string(latest);
            if let Some(r) = latest {
                self.release_selected(r.clone())
            } else {
                warn!("No latest release found in releases");
                None
            }
        } else {
            warn!("Release updated but no releases");
            None
        }
    }

    fn release_selected(&mut self, release: Release) -> Option<AppMessage> {
        // Download the release
        if let Some(board) = self.selected_hw_info.board
            && let Some(mcu) = self.selected_hw_info.mcu_variant
        {
            Some(AppMessage::Studio(StudioMessage::DownloadRelease(
                release, board, mcu,
            )))
        } else {
            warn!("Board or MCU not selected, cannot download firmware");
            None
        }
    }

    fn model_selected(&mut self, model: Model) {
        self.selected_hw_info.model = Some(model);
        self.selected_hw_info.board = None;
        self.selected_hw_info.mcu_variant = None;
        self.mcu_variants = None;
    }

    fn board_selected(&mut self, runtime_info: &RuntimeInfo, board: Board) -> Option<AppMessage> {
        self.selected_hw_info.board = Some(board);
        let mut vars = Vec::new();
        for var in MCU_VARIANTS {
            if board.mcu_family() == var.family() {
                vars.push(*var);
            }
        }
        self.mcu_variants = Some(vars);

        // Special case the Fire boards
        if board.mcu_family() == Family::Rp2350 {
            self.mcu_selected(McuVariant::RP2350);
            self.select_latest_release(runtime_info.releases())
        } else {
            None
        }
    }

    fn mcu_selected(&mut self, mcu: McuVariant) {
        self.selected_hw_info.mcu_variant = Some(mcu);
    }

    fn hardware_selected(&self) -> bool {
        self.selected_hw_info.is_complete()
    }

    pub fn view<'a>(&'a self, runtime_info: &'a RuntimeInfo) -> iced::Element<'a, AppMessage> {
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
            columns = columns
                .push(self.firmware_row(runtime_info))
                .push(Style::horiz_line());
        }

        columns.spacing(20).into()
    }

    fn firmware_row<'a>(&'a self, runtime_info: &'a RuntimeInfo) -> iced::Element<'a, AppMessage> {
        // Create release selection row
        if let Some(releases) = &runtime_info.releases() {
            let latest = releases.latest();

            let selected_release = if let Some(r) = runtime_info.selected_firmware() {
                Some(r)
            } else {
                releases.release_from_string(latest)
            };

            let mut rows = row![
                Style::text_h3("Select Firmware Release"),
                Style::pick_list_small(releases.releases().as_slice(), selected_release, |r| {
                    AppMessage::Create(Message::ReleaseSelected(r))
                })
            ];

            // Show if release has been downloaded
            if let Some(fw_len) = runtime_info.firmware_len() {
                // split into three rows, with number of bytes gold
                let downloaded_row = row![
                    Style::text_small("(downloaded: "),
                    Style::text_small(format!("{}", fw_len)).color(Style::COLOUR_DARK_GOLD),
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
        let model_picker =
            Style::pick_list_small(MODELS.as_slice(), self.selected_hw_info.model, |model| {
                AppMessage::Create(Message::ModelSelected(model))
            });
        let model_picker = row![Style::text_body("Model:"), model_picker,]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center);

        // Set up board picker
        let board_values = if let Some(model) = self.selected_hw_info.model {
            model.boards()
        } else {
            &[]
        };
        let board_picker =
            Style::pick_list_small(board_values, self.selected_hw_info.board, |board| {
                AppMessage::Create(Message::BoardSelected(board))
            });
        let board_picker = row![Style::text_body("Board:"), board_picker,]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center);

        // Set up MCU picker
        let mcu_values = if let Some(vars) = &self.mcu_variants {
            vars.as_slice()
        } else {
            &[]
        };
        let mcu_picker =
            Style::pick_list_small(mcu_values, self.selected_hw_info.mcu_variant, |mcu| {
                AppMessage::Create(Message::McuSelected(mcu))
            });
        let mcu_picker = row![Style::text_body("MCU:"), mcu_picker,]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center);

        row![model_picker, board_picker, mcu_picker]
            .spacing(20)
            .into()
    }

    fn board_description_row(&self) -> iced::Element<'_, AppMessage> {
        if self.hardware_selected() {
            let hw_info_row = Style::hw_info_row(
                None,
                self.selected_hw_info.model,
                self.selected_hw_info.board,
                self.selected_hw_info.mcu_variant,
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

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
