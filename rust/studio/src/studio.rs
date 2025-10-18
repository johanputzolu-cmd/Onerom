// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use iced::widget::Row;
use iced::{Element, Subscription, Task, time};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::time::Duration;

use onerom_config::fw::FirmwareVersion;
use onerom_config::hw::Board;
use onerom_config::mcu::Variant as McuVariant;
use onerom_fw::net::{Release, Releases};

use crate::analyse::Analyse;
use crate::app::AppMessage;
use crate::create::{Create, Message as CreateMessage};
use crate::hw::HardwareInfo;
use crate::log::Log;
use crate::style::Style;
use crate::task_from_msg;

const RELEASES_RETRY_SHORT: Duration = Duration::from_secs(10);
const RELEASES_RETRY_LONG: Duration = Duration::from_secs(60);

/// Messages for main window
#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(StudioTab),
    HardwareInfo(Option<HardwareInfo>),
    FetchReleases,
    Releases(Releases),
    DownloadRelease(Release, Board, McuVariant),
    ReleaseDownloaded(Vec<u8>),
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::TabSelected(tab) => write!(f, "TabSelected({tab})"),
            Message::HardwareInfo(info) => write!(f, "HardwareInfo({info:?})"),
            Message::FetchReleases => write!(f, "FetchReleases"),
            Message::Releases(releases) => write!(f, "Releases({})  ", releases.releases_str()),
            Message::DownloadRelease(release, board, mcu) => {
                write!(f, "DownloadRelease({}, {board}, {mcu})", release.version)
            }
            Message::ReleaseDownloaded(data) => {
                write!(f, "ReleaseDownloaded({} bytes)", data.len())
            }
        }
    }
}

/// Tabs for main window
#[derive(Debug, Default, Clone, PartialEq)]
pub enum StudioTab {
    #[default]
    Analyse,
    Create,
    Log,
}

impl std::fmt::Display for StudioTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StudioTab::Create => write!(f, "Create"),
            StudioTab::Analyse => write!(f, "Analyse"),
            StudioTab::Log => write!(f, "Log"),
        }
    }
}

impl StudioTab {
    /// Get the tab name
    pub fn name(&self) -> &str {
        match self {
            StudioTab::Create => Create::top_level_button_name(),
            StudioTab::Analyse => Analyse::top_level_button_name(),
            StudioTab::Log => Log::top_level_button_name(),
        }
    }

    /// Create the tab buttons
    ///
    /// Returns a Vec of Elements, so they can be easily added to a Row
    pub fn buttons(active: &StudioTab) -> Vec<Element<'_, AppMessage>> {
        let mut buttons = Vec::new();
        for tab in vec![StudioTab::Analyse, StudioTab::Create, StudioTab::Log] {
            let active = *active == tab;
            let on_press = if active {
                None
            } else {
                Some(AppMessage::Studio(Message::TabSelected(tab.clone())))
            };
            let button = Style::text_button(tab.name(), on_press, active);
            buttons.push(button.into());
        }
        buttons
    }
}

/// Contains information retrieved/computed at runtime
#[derive(Debug, Clone, Default)]
pub struct RuntimeInfo {
    // One ROM releases retrieved from network
    releases: Option<Releases>,

    // Detected or selected hardware info
    hw_info: Option<HardwareInfo>,

    // Downloaded firmware image
    firmware: Option<Vec<u8>>,

    // Selected firmware
    selected_firmware: Option<Release>,
}

impl RuntimeInfo {
    pub fn releases(&self) -> Option<&Releases> {
        self.releases.as_ref()
    }

    fn set_releases(&mut self, releases: Releases) {
        self.releases = Some(releases);
    }

    pub fn hw_info(&self) -> Option<&HardwareInfo> {
        self.hw_info.as_ref()
    }

    fn set_hw_info(&mut self, hw_info: Option<HardwareInfo>) {
        self.hw_info = hw_info;
    }

    #[allow(dead_code)]
    pub fn firmware(&self) -> Option<&Vec<u8>> {
        self.firmware.as_ref()
    }

    pub fn firmware_len(&self) -> Option<usize> {
        self.firmware.as_ref().map(|f| f.len())
    }

    fn set_firmware(&mut self, firmware: Vec<u8>) {
        self.firmware = Some(firmware);
    }

    fn clear_firmware(&mut self) {
        self.firmware = None;
    }

    pub fn selected_firmware(&self) -> Option<&Release> {
        self.selected_firmware.as_ref()
    }

    fn set_selected_firmware(&mut self, release: Release) {
        self.selected_firmware = Some(release);
    }
}

/// Main application state
#[derive(Debug, Default, Clone)]
pub struct Studio {
    active_tab: StudioTab,
    runtime_info: RuntimeInfo,
}

impl Studio {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_tab(&self) -> &StudioTab {
        &self.active_tab
    }

    pub fn runtime_info(&self) -> &RuntimeInfo {
        &self.runtime_info
    }

    pub fn update(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            Message::HardwareInfo(info) => {
                self.runtime_info.set_hw_info(info.clone());

                // Share with Create
                task_from_msg!(CreateMessage::DetectedHardwareInfo)
            }
            Message::FetchReleases => Task::future(Self::fetch_releases_async()),
            Message::Releases(releases) => {
                self.runtime_info.set_releases(releases.clone());
                Task::done(CreateMessage::ReleasesUpdated.into())
            }
            Message::DownloadRelease(release, board, mcu) => {
                self.download_release(release, board, mcu)
            }
            Message::ReleaseDownloaded(data) => {
                self.runtime_info.set_firmware(data.clone());
                Task::none()
            }
        }
    }

    async fn fetch_releases_async() -> AppMessage {
        match Releases::from_network_async().await {
            Ok(releases) => AppMessage::Studio(Message::Releases(releases)),
            Err(_) => {
                warn!("Failed to fetch releases from network");
                AppMessage::Nop
            }
        }
    }

    fn download_release(
        &mut self,
        release: Release,
        board: Board,
        mcu: McuVariant,
    ) -> Task<AppMessage> {
        self.runtime_info.clear_firmware();
        self.runtime_info.set_selected_firmware(release.clone());

        // Check we have Releases
        let releases = if let Some(releases) = self.runtime_info.releases() {
            releases.clone()
        } else {
            error!("No releases available in Studio, cannot download");
            return Task::none();
        };

        // Get the firmware version
        let Ok(fw_ver) = release.firmware_version() else {
            warn!("No firmware version {release} found, cannot download");
            return Task::none();
        };

        // Download the firmware
        Task::future(Self::download_release_async(releases, fw_ver, board, mcu))
    }

    async fn download_release_async(
        releases: Releases,
        fw_ver: FirmwareVersion,
        board: Board,
        mcu: McuVariant,
    ) -> AppMessage {
        // Download the firmware
        match releases
            .download_firmware_async(&fw_ver, &board, &mcu)
            .await
        {
            Ok(data) => Message::ReleaseDownloaded(data).into(),
            Err(e) => {
                warn!("Failed to download firmware: {}", e);
                AppMessage::Nop
            }
        }
    }

    pub fn top_level_buttons(&self) -> iced::Element<'_, AppMessage> {
        let buttons = StudioTab::buttons(&self.active_tab());
        Row::with_children(buttons).spacing(20).padding(10).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let check_releases_duration = if self.runtime_info.releases().is_some() {
            RELEASES_RETRY_LONG
        } else {
            RELEASES_RETRY_SHORT
        };
        time::every(check_releases_duration).map(|_| Message::FetchReleases)
    }
}
