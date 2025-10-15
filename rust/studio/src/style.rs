// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Style constants and objects

use iced::border::Radius;
use iced::overlay::menu;
use iced::theme::Theme;
use iced::widget;
use iced::widget::text::{Rich, Span, Text};
use iced::widget::{
    PickList, Row, Scrollable, Space, button, column, container, pick_list, row, scrollable, text,
};
use iced::{Background, Border, Element, Length, Shadow};
use onerom_config::fw::FirmwareVersion;
use onerom_config::hw::{Board, Model};
use onerom_config::mcu::Variant as McuVariant;
use std::borrow::Borrow;

use crate::app::Message as AppMessage;

/// Iced theme to use
pub const ICED_THEME: iced::Theme = iced::Theme::Dark;

// Colours

/// Assets
const FONT_MICHROMA_BYTES: &[u8] = include_bytes!("../fonts/Michroma-Regular.ttf");
const FONT_COURIER_REG_BYTES: &[u8] = include_bytes!("../fonts/CourierPrime-Regular.ttf");
const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.png");

pub fn font_michroma_bytes() -> &'static [u8] {
    FONT_MICHROMA_BYTES
}
pub fn font_courier_reg_bytes() -> &'static [u8] {
    FONT_COURIER_REG_BYTES
}

pub fn icon() -> iced::window::Icon {
    iced::window::icon::from_file_data(ICON_BYTES, None).expect("Failed to load icon")
}

/// Style specific messages
#[derive(Debug, Clone)]
pub enum Message {
    ClickLink(Link),
}

/// Supported links
#[derive(Debug, Clone)]
pub enum Link {
    OneRom,
    PiersRocks,
}

impl Link {
    const fn url(&self) -> &'static str {
        match self {
            Link::OneRom => "https://onerom.org",
            Link::PiersRocks => "https://piers.rocks",
        }
    }
}

/// One ROM Studio styles
pub struct Style<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

#[allow(dead_code)]
impl<'a> Style<'a> {
    /// #ffb700
    pub const COLOUR_GOLD: iced::Color = iced::Color::from_rgb(1.0, 0.72, 0.0);

    /// #cc9200
    pub const COLOUR_DARK_GOLD: iced::Color = iced::Color::from_rgb(0.8, 0.57, 0.0);

    /// #e2e8f0
    pub const COLOUR_TEXT: iced::Color = iced::Color::from_rgb(0.89, 0.91, 0.94);

    /// #9a9aa8
    pub const COLOUR_TEXT_DIM: iced::Color = iced::Color::from_rgb(0.6, 0.6, 0.66);

    /// #181820
    pub const COLOUR_BACKGROUND: iced::Color = iced::Color::from_rgb(0.09, 0.09, 0.10);

    /// #4a4a52
    pub const COLOUR_BORDER: iced::Color = iced::Color::from_rgb(0.29, 0.29, 0.32);

    /// #4a4a52
    pub const COLOUR_DISABLED: iced::Color = iced::Color::from_rgb(0.29, 0.29, 0.32);

    /// Button text colour
    pub const COLOUR_BUTTON_TEXT: iced::Color = Self::COLOUR_BACKGROUND;

    /// Font sizes
    pub const FONT_SIZE_H1: u16 = 32;
    pub const FONT_SIZE_H2: u16 = 26;
    pub const FONT_SIZE_H3: u16 = 20;
    pub const FONT_SIZE_BODY: u16 = 16;
    pub const FONT_SIZE_SMALL: u16 = 14;
    pub const FONT_SIZE_EXTRA_SMALL: u16 = 12;

    /// Fonts
    pub const FONT_MICHROMA: iced::Font = iced::Font::with_name("Michroma");
    pub const FONT_COURIER_REG: iced::Font = iced::Font::with_name("Courier Prime");

    // Button styles
    const BUTTON_RADIUS: Radius = Radius {
        top_left: 4.0,
        top_right: 4.0,
        bottom_left: 4.0,
        bottom_right: 4.0,
    };
    const BUTTON_BORDER: Border = Border {
        color: Self::COLOUR_BORDER,
        width: 1.0,
        radius: Self::BUTTON_RADIUS,
    };
    const BUTTON_SHADOW: Shadow = Shadow {
        color: iced::Color::BLACK,
        offset: iced::Vector::new(0.0, 2.0),
        blur_radius: 4.0,
    };

    // Prevent instantiation
    fn new() -> Self {
        Style {
            _marker: std::marker::PhantomData,
        }
    }

    /// Handle style messages
    pub fn update(message: Message) -> iced::Task<Message> {
        match message {
            Message::ClickLink(link) => {
                if let Err(e) = open::that(link.url()) {
                    eprintln!("Failed to open link {}: {}", link.url(), e);
                }
            }
        }
        iced::Task::none()
    }

    pub fn text_studio_h1() -> Rich<'a, AppMessage> {
        widget::rich_text![
            Span::new("One R").color(Self::COLOUR_TEXT),
            Span::new("O").color(Self::COLOUR_GOLD),
            Span::new("M Studio").color(Self::COLOUR_TEXT),
        ]
        .font(Self::FONT_MICHROMA)
        .size(Self::FONT_SIZE_H1)
    }

    pub fn text_h2(content: impl ToString) -> Text<'a> {
        text(content.to_string())
            .size(Self::FONT_SIZE_H2)
            .color(Self::COLOUR_TEXT)
    }

    pub fn text_h3(content: impl ToString) -> Text<'a> {
        text(content.to_string())
            .size(Self::FONT_SIZE_H3)
            .color(Self::COLOUR_TEXT)
    }

    pub fn text_body(content: impl ToString) -> Text<'a> {
        text(content.to_string())
            .size(Self::FONT_SIZE_BODY)
            .color(Self::COLOUR_TEXT)
    }

    pub fn text_small(content: impl ToString) -> Text<'a> {
        text(content.to_string())
            .size(Self::FONT_SIZE_SMALL)
            .color(Self::COLOUR_TEXT)
    }

    pub fn text_extra_small(content: impl ToString) -> Text<'a> {
        text(content.to_string())
            .size(Self::FONT_SIZE_EXTRA_SMALL)
            .color(Self::COLOUR_TEXT)
    }

    pub fn text_button(
        content: impl ToString,
        on_press: Option<AppMessage>,
        highlighted: bool,
    ) -> widget::Button<'a, AppMessage> {
        // Set up the styles
        let (text_color, background) = if highlighted {
            (
                Style::COLOUR_BUTTON_TEXT,
                Some(Background::Color(Style::COLOUR_GOLD)),
            )
        } else {
            (
                Style::COLOUR_TEXT,
                Some(Background::Color(Style::COLOUR_DISABLED)),
            )
        };

        let text = Self::text_body(content.to_string()).color(Self::COLOUR_BUTTON_TEXT);
        let mut button = button(text)
            .style(move |_, _| button::Style {
                background,
                text_color,
                border: Self::BUTTON_BORDER,
                shadow: Self::BUTTON_SHADOW,
            })
            .padding([10, 20]);

        if let Some(msg) = on_press {
            button = button.on_press(msg);
        }

        button
    }

    pub fn horiz_line() -> widget::Container<'a, AppMessage> {
        widget::container(widget::horizontal_space())
            .height(1.0)
            .width(iced::Length::Fill)
            .style(|_| widget::container::Style {
                background: Some(Background::Color(Style::COLOUR_BORDER)),
                ..widget::container::Style::default()
            })
    }

    pub fn box_scrollable(content: impl ToString, height: f32) -> Scrollable<'a, AppMessage> {
        let text = Self::text_small(content.to_string()).font(Self::FONT_COURIER_REG);
        scrollable(text)
            .height(Length::Fixed(height))
            .width(Length::Fill)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .into()
    }

    pub fn blank_space() -> Space {
        Space::with_height(Length::Fill)
    }

    fn link_button_style() -> button::Style {
        button::Style {
            background: None,
            text_color: Self::COLOUR_GOLD,
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
    fn link(content: impl ToString, size: u16, link: Link) -> widget::Button<'a, AppMessage> {
        let text = Self::text_body(content.to_string())
            .size(size)
            .color(Self::COLOUR_GOLD);

        button(text)
            .style(|_, _| Self::link_button_style())
            .padding(0)
            .on_press(AppMessage::Style(Message::ClickLink(link)))
    }

    fn footer_1_left() -> Element<'a, AppMessage> {
        Self::link("One ROM", Self::FONT_SIZE_BODY, Link::OneRom).into()
    }

    fn footer_1_right() -> Rich<'a, AppMessage> {
        widget::rich_text![
            Span::new("Copyright © 2").color(Self::COLOUR_TEXT),
            Span::new("0").color(Self::COLOUR_GOLD),
            Span::new("25").color(Self::COLOUR_TEXT),
        ]
        .font(Self::FONT_MICHROMA)
        .size(Self::FONT_SIZE_BODY)
    }

    fn footer_2_left() -> Rich<'a, AppMessage> {
        widget::rich_text![
            Span::new("the most flexible retro R").color(Self::COLOUR_TEXT),
            Span::new("O").color(Self::COLOUR_GOLD),
            Span::new("M replacement").color(Self::COLOUR_TEXT),
        ]
        .font(Self::FONT_MICHROMA)
        .size(Self::FONT_SIZE_BODY)
    }

    fn footer_2_right() -> Element<'a, AppMessage> {
        Self::link("piers.rocks", Self::FONT_SIZE_BODY, Link::PiersRocks).into()
    }

    fn footer_row_1() -> Row<'a, AppMessage> {
        let left = Self::footer_1_left();
        let right = Self::footer_1_right();
        Row::new()
            .push(left)
            .push(Space::with_width(Length::Fill))
            .push(right)
    }

    fn footer_row_2() -> Row<'a, AppMessage> {
        let left = Self::footer_2_left();
        let right = Self::footer_2_right();
        Row::new()
            .push(left)
            .push(Space::with_width(Length::Fill))
            .push(right)
    }

    pub fn footer() -> Element<'a, AppMessage> {
        column![Self::footer_row_1(), Self::footer_row_2(),]
            .spacing(5)
            .into()
    }

    pub fn container(
        content: impl Into<Element<'a, AppMessage>>,
    ) -> widget::Container<'a, AppMessage> {
        container(content)
            .padding(10)
            .style(|_| widget::container::Style {
                background: Some(Background::Color(Style::COLOUR_BACKGROUND)),
                border: Self::BUTTON_BORDER,
                ..widget::container::Style::default()
            })
    }

    pub fn pick_list<T, L, V>(
        options: L,
        selected: Option<V>,
        on_selected: impl Fn(T) -> AppMessage + 'a,
    ) -> PickList<'a, T, L, V, AppMessage>
    where
        T: ToString + PartialEq + Clone + 'a,
        L: Borrow<[T]> + 'a,
        V: Borrow<T> + 'a,
    {
        pick_list(options, selected, on_selected)
            .style(|_theme: &Theme, status| pick_list::Style {
                background: if matches!(status, pick_list::Status::Hovered) {
                    Background::Color(Self::COLOUR_GOLD)
                } else {
                    Background::Color(Self::COLOUR_BACKGROUND)
                },
                text_color: if matches!(status, pick_list::Status::Hovered) {
                    Self::COLOUR_BACKGROUND
                } else {
                    Self::COLOUR_TEXT
                },
                placeholder_color: Self::COLOUR_TEXT_DIM,
                handle_color: Self::COLOUR_TEXT,
                border: Self::BUTTON_BORDER,
            })
            .menu_style(|_theme: &Theme| menu::Style {
                background: Background::Color(Self::COLOUR_BACKGROUND),
                border: Self::BUTTON_BORDER,
                text_color: Self::COLOUR_TEXT,
                selected_text_color: Self::COLOUR_BACKGROUND,
                selected_background: Background::Color(Self::COLOUR_GOLD),
            })
    }

    pub fn hw_info_row(
        version: Option<FirmwareVersion>,
        model: Option<Model>,
        board: Option<Board>,
        mcu: Option<McuVariant>,
        board_long: bool,
    ) -> Element<'a, AppMessage> {
        let fw = if let Some(version) = version {
            // Firmware version
            let fw_h = Style::text_small("Firmware:");
            let fw_str = format!(
                "v{}.{}.{}",
                version.major(),
                version.minor(),
                version.patch(),
            );
            let fw = Style::text_small(fw_str).color(Style::COLOUR_DARK_GOLD);
            Some((fw_h, fw))
        } else {
            None
        };

        // Model
        let model_h = Style::text_small("Model:");
        let model = Style::text_small(
            model
                .as_ref()
                .map_or("unknown".to_string(), |m| m.name().to_string()),
        )
        .color(Style::COLOUR_DARK_GOLD);

        // Board
        let board_h = Style::text_small("Board:");
        let board = Style::text_small(board.as_ref().map_or("unknown".to_string(), |b| {
            if board_long {
                b.description()
            } else {
                b.name()
            }
            .to_string()
        }))
        .color(Style::COLOUR_DARK_GOLD);

        // MCU
        let mcu_h = Style::text_small("MCU:");
        let mcu = Style::text_small(
            mcu.as_ref()
                .map_or("unknown".to_string(), |m| m.to_string()),
        )
        .color(Style::COLOUR_DARK_GOLD);

        let model_row = row![model_h, model].spacing(5);
        let board_row = row![board_h, board].spacing(5);
        let mcu_row = row![mcu_h, mcu].spacing(5);

        if let Some((fw_h, fw)) = fw {
            let fw_row = row![fw_h, fw].spacing(5);
            row![fw_row, model_row, board_row, mcu_row]
        } else {
            row![model_row, board_row, mcu_row]
        }
        .spacing(20)
        .into()
    }
}
