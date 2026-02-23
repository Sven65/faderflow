use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length};

use crate::ui::app::Message;

#[derive(Debug, Clone, PartialEq)]
pub enum NoDevicesReason {
    NoneFound,
    Lost { port_name: String, retry_in_secs: u64 },
}

pub fn view(reason: &NoDevicesReason) -> Element<Message> {
    let (headline, sub, show_countdown) = match reason {
        NoDevicesReason::NoneFound => (
            "No FaderFlow devices found",
            "Make sure your device is plugged in and powered on.".to_string(),
            false,
        ),
        NoDevicesReason::Lost { port_name, retry_in_secs } => (
            "Device disconnected",
            format!("{port_name} stopped responding. Retrying in {retry_in_secs}s…"),
            true,
        ),
    };

    let icon = text("⚡").size(48);

    let title = text(headline).size(22);

    let subtitle = text(sub)
        .size(14)
        .color(iced::Color::from_rgb(0.6, 0.6, 0.6));

    let rescan_btn = button(
        text("Rescan now").size(14)
    )
        .on_press(Message::StartScan)
        .padding([8, 20]);

    let mut col = column![
        icon,
        title,
        subtitle,
        rescan_btn,
    ]
        .spacing(12)
        .align_x(Alignment::Center)
        .width(Length::Fixed(400.0));

    if show_countdown {
        col = col.push(
            text("(or plug in your device to auto-detect)")
                .size(12)
                .color(iced::Color::from_rgb(0.45, 0.45, 0.45))
        );
    }

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}