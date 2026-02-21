use crate::ui::app::Message;
use iced::widget::{column, text};
use iced::Element;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn view<'a>() -> Element<'a, Message> {
    column![
        text("About FaderFlow").size(24),
        text(format!("Version {}", VERSION)).size(16),
        text("A motorized volume controller with individual displays").size(14),
        text("").size(10),
        text("Created by Mackan").size(14),
    ]
        .spacing(10)
        .into()
}
