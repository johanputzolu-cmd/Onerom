// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

use iced::widget::image::Handle;
use tiny_skia::Pixmap;
use usvg::{Options, Transform, Tree};

use crate::style::Style;

const NETWORK_OK_SVG: &str = include_str!("../assets/network_ok.svg");
const NETWORK_ERROR_SVG: &str = include_str!("../assets/network_error.svg");

/// Stored image (pictures/icons) resources
pub struct Images {
    network_ok: Handle,
    network_error: Handle,
}

impl Images {
    /// Initialize the images
    pub fn new() -> Self {
        let network_ok = Self::svg_to_image(NETWORK_OK_SVG, 24, 24);
        let network_error = Self::svg_to_image(NETWORK_ERROR_SVG, 24, 24);

        Self {
            network_ok,
            network_error,
        }
    }

    pub fn icon_network_connected(&self) -> &Handle {
        &self.network_ok
    }

    pub fn icon_network_disconnected(&self) -> &Handle {
        &self.network_error
    }

    fn svg_to_image(svg_str: &str, width: u32, height: u32) -> Handle {
        // Set colours
        let svg_str = svg_str
            .replace("primaryColour", Style::COLOUR_TEXT_DIM_STR)
            .replace("errorColour", Style::COLOUR_ERROR_STR);

        // Parse SVG
        let opts = Options::default();
        let tree = Tree::from_str(&svg_str, &opts).unwrap();
        
        // Create pixmap to render into
        let mut pixmap = Pixmap::new(width, height).unwrap();
        
        // Render
        let tree_size = tree.size();
        let scale_x = width as f32 / tree_size.width();
        let scale_y = height as f32 / tree_size.height();
        let transform = Transform::from_scale(scale_x, scale_y);
        
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        
        // Convert RGBA bytes to iced Handle
        Handle::from_rgba(width, height, pixmap.data().to_vec())
    }
}