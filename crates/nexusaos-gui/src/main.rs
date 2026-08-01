pub mod app;
pub mod view;
pub mod terminal;
pub mod theme;

use iced::Font;
use app::NexusApp;

fn main() -> iced::Result {
    iced::application(NexusApp::new, NexusApp::update, NexusApp::view)
        .title(NexusApp::title)
        .subscription(NexusApp::subscription)
        .theme(NexusApp::theme)
        .default_font(Font::MONOSPACE)
        .antialiasing(true)
        .run()
}
