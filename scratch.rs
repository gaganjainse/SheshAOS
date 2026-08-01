use iced::{Task, Element, Theme, Font, Subscription};
pub struct MyApp;
#[derive(Debug, Clone)]
pub enum Message {}
impl MyApp {
    pub fn new() -> (Self, Task<Message>) { (MyApp, Task::none()) }
    pub fn title(&self) -> String { "Title".into() }
    pub fn update(&mut self, _msg: Message) -> Task<Message> { Task::none() }
    pub fn view(&self) -> Element<Message> { iced::widget::text("Hello").into() }
    pub fn theme(&self) -> Theme { Theme::CatppuccinMocha }
    pub fn subscription(&self) -> Subscription<Message> { Subscription::none() }
}
fn main() -> iced::Result {
    iced::application(MyApp::title, MyApp::update, MyApp::view)
        .subscription(MyApp::subscription)
        .theme(MyApp::theme)
        .default_font(Font::MONOSPACE)
        .antialiasing(true)
        .run_with(MyApp::new)
}
