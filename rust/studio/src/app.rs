// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use futures_timer::Delay;
use iced::Length;
use iced::Task;
use iced::widget::{Space, column, row};
use std::time::Duration;

use onerom_config::hw::Board;
use onerom_config::mcu::Variant as McuVariant;
use onerom_fw::net::{Release, Releases};

use crate::analyse::{Analyse, HardwareInfo, Message as AnalyseMessage};
use crate::create::{Create, Message as CreateMessage};
use crate::device::{Device, Message as DeviceMessage};
use crate::style::{Message as StyleMessage, Style};

const DEFAULT_TAB: StudioTab = StudioTab::Analyse;
const RELEASES_RETRY_SECS: Duration = Duration::from_secs(10);

/// Kicks off any startup tasks fo the app
///
/// - Select the default top-level tab
/// - Fetch One ROM releases from the network
pub fn startup_task() -> Task<Message> {
    Task::batch([
        Task::done(Message::Studio(StudioMessage::TabSelected(DEFAULT_TAB))),
        Task::done(Message::Studio(StudioMessage::FetchReleases(false))),
        Task::done(Message::Device(DeviceMessage::DetectProbe(false))),
    ])
    .into()
}

/// Top level Message enum
#[derive(Debug, Clone)]
pub enum Message {
    Analyse(AnalyseMessage),
    Create(CreateMessage),
    Device(DeviceMessage),
    Studio(StudioMessage),
    Style(StyleMessage),
}

impl From<StudioMessage> for Message {
    fn from(msg: StudioMessage) -> Self {
        Message::Studio(msg)
    }
}

impl From<CreateMessage> for Message {
    fn from(msg: CreateMessage) -> Self {
        Message::Create(msg)
    }
}

impl From<AnalyseMessage> for Message {
    fn from(msg: AnalyseMessage) -> Self {
        Message::Analyse(msg)
    }
}

impl From<StyleMessage> for Message {
    fn from(msg: StyleMessage) -> Self {
        Message::Style(msg)
    }
}

/// Messages for main window
#[derive(Debug, Clone)]
pub enum StudioMessage {
    TabSelected(StudioTab),
    HardwareInfo(HardwareInfo),
    FetchReleases(bool),
    Releases(Releases),
    DownloadRelease(Release, Board, McuVariant),
    ReleaseDownloaded(Vec<u8>),
}

/// Tabs for main window
#[derive(Debug, Clone)]
pub enum StudioTab {
    Create,
    Analyse,
}

/// Main application state
pub struct Studio {
    active_tab: StudioTab,
    programmer: Create,
    firmware: Analyse,
    device: Device,
    releases: Option<Releases>,
}

impl Studio {
    pub fn new() -> Self {
        Self {
            active_tab: DEFAULT_TAB,
            programmer: Create::new(),
            firmware: Analyse::new(),
            device: Device::new(),
            releases: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Analyse(fw_msg) => self.firmware.update(fw_msg).map(|m| m.into()),
            Message::Device(dev_msg) => self.device.message(dev_msg).map(|m| m.into()),
            Message::Create(prog_msg) => self.programmer.update(prog_msg).map(|m| m.into()),
            Message::Studio(studio_msg) => self.message(studio_msg).map(|m| m.into()),
            Message::Style(style_msg) => Style::update(style_msg).map(|m| m.into()),
        }
    }

    fn message(&mut self, message: StudioMessage) -> Task<Message> {
        match message {
            StudioMessage::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            StudioMessage::HardwareInfo(info) => Task::batch([
                Task::done(Message::Analyse(AnalyseMessage::HardwareInfo(info.clone()))),
                Task::done(Message::Create(CreateMessage::HardwareInfo(info.clone()))),
            ]),
            StudioMessage::FetchReleases(delay) => self.fetch_releases(delay),
            StudioMessage::Releases(releases) => {
                self.releases = Some(releases.clone());
                Task::done(Message::Create(CreateMessage::Releases(releases)))
            }
            StudioMessage::DownloadRelease(release, board, mcu) => {
                self.download_release(release, board, mcu)
            }
            StudioMessage::ReleaseDownloaded(data) => {
                Task::done(Message::Create(CreateMessage::ReleaseDownloaded(data)))
            }
        }
    }

    fn fetch_releases(&self, delay: bool) -> Task<Message> {
        Task::perform(
            async move {
                if delay {
                    Delay::new(RELEASES_RETRY_SECS).await;
                }
                match Releases::from_network_async().await {
                    Ok(releases) => Message::Studio(StudioMessage::Releases(releases)),
                    Err(_) => Message::Studio(StudioMessage::FetchReleases(true)),
                }
            },
            |msg| msg,
        )
    }

    fn download_release(&self, release: Release, board: Board, mcu: McuVariant) -> Task<Message> {
        // Check we have Releases
        if self.releases.is_none() {
            eprintln!("No releases available in Studio, cannot download");
            return Task::none();
        }
        let releases = self.releases.as_ref().unwrap().clone();

        // Get the firmware version
        let fw_ver = release.firmware_version();
        let Ok(fw_ver) = fw_ver else {
            eprintln!("No firmware version {release} found, cannot download");
            return Task::none();
        };

        // Download the firmware
        Task::perform(
            async move {
                match releases
                    .download_firmware_async(&fw_ver, &board, &mcu)
                    .await
                {
                    Ok(data) => Message::Studio(StudioMessage::ReleaseDownloaded(data)),
                    Err(e) => Message::Analyse(AnalyseMessage::FileLoaded(Err(e.to_string()))),
                }
            },
            |msg| msg,
        )
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        column![
            self.title_row(),
            self.top_level_buttons(),
            Style::horiz_line(),
            self.content_row(),
            Style::blank_space(),
            Style::horiz_line(),
            Style::footer(),
        ]
        .padding([20, 20])
        .spacing(20)
        .into()
    }

    fn title_row(&self) -> iced::Element<'_, Message> {
        let probe_elements = self.device.probe_pick_list();
        row![
            Style::text_studio_h1(),
            Space::with_width(Length::Fill),
            probe_elements
        ]
        .spacing(20)
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn top_level_buttons(&self) -> iced::Element<'_, Message> {
        let (prog_button, fw_button) = match self.active_tab {
            StudioTab::Create => (
                Style::text_button(
                    Analyse::top_level_button_name(),
                    Some(Message::Studio(StudioMessage::TabSelected(
                        StudioTab::Analyse,
                    ))),
                    false,
                ),
                Style::text_button(Create::top_level_button_name(), None, true),
            ),
            StudioTab::Analyse => (
                Style::text_button(Analyse::top_level_button_name(), None, true),
                Style::text_button(
                    Create::top_level_button_name(),
                    Some(Message::Studio(StudioMessage::TabSelected(
                        StudioTab::Create,
                    ))),
                    false,
                ),
            ),
        };

        row![prog_button, fw_button].spacing(20).padding(10).into()
    }

    #[allow(dead_code)]
    fn content_title(&self) -> iced::Element<'_, Message> {
        let title = match self.active_tab {
            StudioTab::Create => Style::text_h2(Create::heading()),
            StudioTab::Analyse => Style::text_h2(Analyse::heading()),
        };

        row![title].into()
    }

    fn content_row(&self) -> iced::Element<'_, Message> {
        match self.active_tab {
            StudioTab::Create => self.programmer.view().map(|m| m.into()),
            StudioTab::Analyse => self.firmware.view().map(|m| m.into()),
        }
    }
}
