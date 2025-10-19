// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use iced::widget::{Space, column, row};
use iced::{Length, Subscription, Task};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::analyse::{Analyse, Message as AnalyseMessage};
use crate::create::{Create, Message as CreateMessage};
use crate::device::{Device, Message as DeviceMessage};
use crate::log::{Level, Log, LogEntry, Message as LogMessage};
use crate::studio::{Message as StudioMessage, RuntimeInfo, Studio, StudioTab};
use crate::style::{Message as StyleMessage, Style};

/// Kicks off any startup tasks fo the app
///
/// - Select the default top-level tab
/// - Fetch One ROM releases from the network
pub fn startup_task() -> Task<AppMessage> {
    Task::batch([
        Task::done(AppMessage::Log(LogMessage::AddEntry(LogEntry::new(
            Level::Info,
            "One ROM Studio started".to_string(),
        )))),
        Task::done(AppMessage::Studio(StudioMessage::FetchReleases)),
        Task::done(AppMessage::Studio(StudioMessage::FetchConfigs)),
        Task::done(AppMessage::Device(DeviceMessage::DetectProbes)),
        Task::done(AppMessage::Device(DeviceMessage::DetectUsbDevices)),
    ])
    .into()
}

/// Top level Message enum
#[derive(Debug, Clone)]
pub enum AppMessage {
    /// Analyse pane messages
    Analyse(AnalyseMessage),

    /// Create pane messages
    Create(CreateMessage),

    /// Device related messages (debug probe and USB operations)
    Device(DeviceMessage),

    /// Log related messages
    Log(LogMessage),

    /// Studio (main application) messages.  May be used for high level
    /// operations, and also for when a single message may need to have
    /// effects on multiple sub-modules.
    Studio(StudioMessage),

    /// Style related messages
    Style(StyleMessage),

    /// No-operation message, where it's easier to return a message than none
    /// at all (for example in match arms).
    #[allow(dead_code)]
    Nop,
}

impl std::fmt::Display for AppMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMessage::Analyse(msg) => write!(f, "Analyse::{msg}"),
            AppMessage::Create(msg) => write!(f, "Create::{msg}"),
            AppMessage::Device(msg) => write!(f, "Device::{msg}"),
            AppMessage::Log(msg) => write!(f, "Log::{msg}"),
            AppMessage::Studio(msg) => write!(f, "Studio::{msg}"),
            AppMessage::Style(msg) => write!(f, "Style::{msg}"),
            AppMessage::Nop => write!(f, "Nop"),
        }
    }
}

/// Main application structure
pub struct App<'a> {
    create: Create,
    analyse: Analyse,
    device: Device,
    log: Log,
    studio: Studio,
    style: Style<'a>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            create: Create::new(),
            analyse: Analyse::new(),
            device: Device::new(),
            style: Style::new(),
            log: Log::new(),
            studio: Studio::new(),
        }
    }

    fn runtime_info(&self) -> &RuntimeInfo {
        self.studio.runtime_info()
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        let runtime_info = self.runtime_info().clone();

        // Log non-log no-op messages
        match &message {
            AppMessage::Nop | AppMessage::Log(_) => {}
            m => trace!("{m}"),
        }
        match message {
            AppMessage::Analyse(fw_msg) => {
                self.analyse.update(&runtime_info, fw_msg).map(|m| m.into())
            }
            AppMessage::Device(dev_msg) => {
                self.device.update(&runtime_info, dev_msg).map(|m| m.into())
            }
            AppMessage::Create(prog_msg) => self
                .create
                .update(&runtime_info, prog_msg)
                .map(|m| m.into()),
            AppMessage::Studio(studio_msg) => self.studio.update(studio_msg).map(|m| m.into()),
            AppMessage::Log(log_msg) => self.log.update(&runtime_info, log_msg).map(|m| m.into()),
            AppMessage::Style(style_msg) => self.style.update(style_msg).map(|m| m.into()),
            AppMessage::Nop => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        let runtime_info = self.runtime_info();

        let top_left_corner =
            column![Style::text_studio_h1(), self.studio.top_level_buttons(),].spacing(20);

        let top_right_corner = self.device.view();

        let top_row = row![
            top_left_corner,
            Space::with_width(Length::Fill),
            top_right_corner,
        ]
        .spacing(20);

        let content_row = match self.studio.active_tab() {
            StudioTab::Analyse => self.analyse.view(runtime_info, &self.device),
            StudioTab::Create => self.create.view(runtime_info),
            StudioTab::Log => self.log.view(runtime_info),
        };

        column![
            top_row,
            Style::horiz_line(),
            content_row,
            Style::blank_space(),
            Style::horiz_line(),
            Style::footer(),
        ]
        .padding([20, 20])
        .spacing(20)
        .into()
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        Subscription::batch(vec![
            self.studio
                .subscription()
                .map(|msg| AppMessage::Studio(msg)),
            self.analyse
                .subscription()
                .map(|msg| AppMessage::Analyse(msg)),
            self.create
                .subscription()
                .map(|msg| AppMessage::Create(msg)),
            self.device
                .subscription()
                .map(|msg| AppMessage::Device(msg)),
            self.log.subscription(),
        ])
    }
}

impl From<StudioMessage> for AppMessage {
    fn from(msg: StudioMessage) -> Self {
        AppMessage::Studio(msg)
    }
}

impl From<CreateMessage> for AppMessage {
    fn from(msg: CreateMessage) -> Self {
        AppMessage::Create(msg)
    }
}

impl From<DeviceMessage> for AppMessage {
    fn from(msg: DeviceMessage) -> Self {
        AppMessage::Device(msg)
    }
}

impl From<AnalyseMessage> for AppMessage {
    fn from(msg: AnalyseMessage) -> Self {
        AppMessage::Analyse(msg)
    }
}

impl From<StyleMessage> for AppMessage {
    fn from(msg: StyleMessage) -> Self {
        AppMessage::Style(msg)
    }
}

impl From<LogMessage> for AppMessage {
    fn from(msg: LogMessage) -> Self {
        AppMessage::Log(msg)
    }
}

// For each variant type
impl From<CreateMessage> for Option<AppMessage> {
    fn from(msg: CreateMessage) -> Self {
        Some(AppMessage::Create(msg))
    }
}

impl From<DeviceMessage> for Option<AppMessage> {
    fn from(msg: DeviceMessage) -> Self {
        Some(AppMessage::Device(msg))
    }
}

impl From<StudioMessage> for Option<AppMessage> {
    fn from(msg: StudioMessage) -> Self {
        Some(AppMessage::Studio(msg))
    }
}

impl From<AnalyseMessage> for Option<AppMessage> {
    fn from(msg: AnalyseMessage) -> Self {
        Some(AppMessage::Analyse(msg))
    }
}

impl From<StyleMessage> for Option<AppMessage> {
    fn from(msg: StyleMessage) -> Self {
        Some(AppMessage::Style(msg))
    }
}

impl From<LogMessage> for Option<AppMessage> {
    fn from(msg: LogMessage) -> Self {
        Some(AppMessage::Log(msg))
    }
}
