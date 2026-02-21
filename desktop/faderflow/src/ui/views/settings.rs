use crate::ui::app::Message;
use iced::widget::{column, text};
use iced::Element;

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("Settings").size(24),
        text("Settings options will go here...").size(16),
    ]
        .spacing(20)
        .into()
}