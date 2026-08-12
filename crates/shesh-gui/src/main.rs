pub mod app;
pub mod terminal;
pub mod theme;
pub mod view;

use app::NexusApp;
use iced::Font;

fn main() -> iced::Result {
    iced::application(NexusApp::new, NexusApp::update, NexusApp::view)
        .title(NexusApp::title)
        .subscription(NexusApp::subscription)
        .theme(NexusApp::theme)
        .default_font(Font::MONOSPACE)
        .antialiasing(true)
        .run()
}
