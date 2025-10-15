// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! One ROM Studio - a GUI application for managing One ROMs

mod analyse;
mod app;
mod create;
mod device;
mod style;

use app::Studio;

// Main - application entry point
fn main() -> iced::Result {
    // Run the application
    iced::application("One ROM Studio", Studio::update, Studio::view)
        .window(window_settings())
        .font(style::font_michroma_bytes())
        .font(style::font_courier_reg_bytes())
        .default_font(style::Style::FONT_MICHROMA)
        .theme(|_| style::ICED_THEME)
        .run_with(|| (Studio::new(), app::startup_task()))
}

// Create the window settings
fn window_settings() -> iced::window::Settings {
    // Create the window settings
    iced::window::Settings {
        size: iced::Size {
            width: 800.0,
            height: 850.0,
        },
        min_size: Some(iced::Size {
            width: 800.0,
            height: 850.0,
        }),
        max_size: None,
        resizable: true,
        decorations: true,
        transparent: false,
        icon: Some(style::icon()),
        position: iced::window::Position::Centered,
        visible: true,
        level: iced::window::Level::Normal,
        exit_on_close_request: true,
        ..Default::default()
    }
}
