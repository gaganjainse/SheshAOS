use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme};

// --- Catppuccin Mocha Palette ---
pub const BASE: Color = Color::from_rgb(24.0 / 255.0, 24.0 / 255.0, 37.0 / 255.0);
pub const MANTLE: Color = Color::from_rgb(17.0 / 255.0, 17.0 / 255.0, 27.0 / 255.0);
pub const CRUST: Color = Color::from_rgb(11.0 / 255.0, 11.0 / 255.0, 16.0 / 255.0);
pub const SURFACE0: Color = Color::from_rgb(49.0 / 255.0, 50.0 / 255.0, 68.0 / 255.0);
pub const SURFACE1: Color = Color::from_rgb(69.0 / 255.0, 71.0 / 255.0, 90.0 / 255.0);
pub const TEXT: Color = Color::from_rgb(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0);
pub const SUBTEXT0: Color = Color::from_rgb(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0);
pub const BLUE: Color = Color::from_rgb(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0);
pub const GREEN: Color = Color::from_rgb(166.0 / 255.0, 227.0 / 255.0, 161.0 / 255.0);
pub const LAVENDER: Color = Color::from_rgb(180.0 / 255.0, 190.0 / 255.0, 254.0 / 255.0);
pub const PEACH: Color = Color::from_rgb(250.0 / 255.0, 179.0 / 255.0, 135.0 / 255.0);

// --- Sidebar: the narrow icon rail on the left ---
pub fn sidebar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CRUST)),
        text_color: Some(SUBTEXT0),
        border: Border { color: SURFACE0, width: 0.0, radius: iced::border::Radius::from(0.0) },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// --- Main content area: wraps the terminal panes ---
pub fn main_area_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BASE)),
        text_color: Some(TEXT),
        border: Border { color: SURFACE0, width: 0.0, radius: iced::border::Radius::from(0.0) },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// --- Tab bar sitting above the terminal ---
pub fn tab_bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(MANTLE)),
        text_color: Some(SUBTEXT0),
        border: Border { color: SURFACE0, width: 0.0, radius: iced::border::Radius::from(0.0) },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// --- Terminal pane: the darkest region where shell output renders ---
pub fn terminal_pane_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(MANTLE)),
        text_color: Some(GREEN),
        border: Border { color: SURFACE0, width: 1.0, radius: iced::border::Radius::from(6.0) },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// --- Status bar at the bottom ---
pub fn status_bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CRUST)),
        text_color: Some(SUBTEXT0),
        border: Border { color: SURFACE0, width: 0.0, radius: iced::border::Radius::from(0.0) },
        shadow: Shadow::default(),
        ..Default::default()
    }
}
