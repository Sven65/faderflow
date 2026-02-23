use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler, Space};
use iced::{Alignment, Color, Element, Length};

use crate::comms::device_info::{DeviceInfo, DeviceStatus};
use crate::ui::app::Message;

pub fn view<'a>(
    devices: &'a [DeviceInfo],
    rename_drafts: &'a [String],
    debug_open: &'a [bool],
) -> Element<'a, Message> {
    let title = text("Connected Devices").size(20);

    let rescan_btn = button(text("🔍 Rescan").size(13))
        .on_press(Message::StartScan)
        .padding([6, 14]);

    let header = row![title, Space::new().width(Length::Fill), rescan_btn]
        .align_y(Alignment::Center);

    let cards: Vec<Element<Message>> = devices
        .iter()
        .enumerate()
        .map(|(i, dev)| device_card(i, dev, &rename_drafts[i], debug_open[i]))
        .collect();

    let content = column![
        header,
        scrollable(column(cards).spacing(12))
    ]
        .spacing(16);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .into()
}

fn device_card<'a>(
    idx: usize,
    dev: &'a DeviceInfo,
    rename_draft: &'a str,
    debug_open: bool,
) -> Element<'a, Message> {
    let status_color = match dev.status {
        DeviceStatus::Connected => Color::from_rgb(0.2, 0.85, 0.4),
        DeviceStatus::Lost      => Color::from_rgb(0.9, 0.3, 0.3),
    };
    let status_label = match dev.status {
        DeviceStatus::Connected => "● Connected",
        DeviceStatus::Lost      => "● Lost",
    };

    // ── Info rows ────────────────────────────────────────────────────────
    let info = column![
        row![
            label("Port"),
            text(&dev.port_name).size(13),
        ].spacing(8),
        row![
            label("UUID"),
            text(dev.uuid_string()).size(13),
        ].spacing(8),
        row![
            label("Firmware"),
            text(dev.version_string()).size(13),
        ].spacing(8),
    ].spacing(6);

    // ── Rename row ───────────────────────────────────────────────────────
    let rename_row = row![
        label("Name"),
        text_input("Custom name…", rename_draft)
            .on_input(move |s| Message::DeviceRenameDraft(idx, s))
            .on_submit(Message::DeviceRenameCommit(idx))
            .size(13)
            .width(Length::Fixed(180.0)),
        button(text("Save").size(12))
            .on_press(Message::DeviceRenameCommit(idx))
            .padding([4, 10]),
    ]
        .spacing(8)
        .align_y(Alignment::Center);

    // ── Debug section ────────────────────────────────────────────────────
    let debug_toggle = row![
        toggler(debug_open)
            .on_toggle(move |_| Message::DeviceToggleDebug(idx))
            .size(16),
        text("Raw debug info").size(12)
            .color(Color::from_rgb(0.55, 0.55, 0.55)),
    ]
        .spacing(6)
        .align_y(Alignment::Center);

    let mut card_col = column![
        // Header: name + status badge
        row![
            text(dev.display_name()).size(15),
            Space::new().width(Length::Fill),
            text(status_label).size(12).color(status_color),
        ]
        .align_y(Alignment::Center),
        info,
        rename_row,
        debug_toggle,
    ]
        .spacing(10);

    if debug_open {
        let raw_uuid = dev.uuid.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        let debug_block = container(
            column![
                text("UUID bytes:").size(11)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
                text(raw_uuid).size(11)
                    .color(Color::from_rgb(0.7, 0.85, 1.0)),
                text(format!("Firmware: {}.{}",
                    dev.version.0, dev.version.1)).size(11)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
                .spacing(4)
        )
            .padding(10)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(
                    Color::from_rgb(0.1, 0.1, 0.1)
                )),
                border: iced::Border {
                    color: Color::from_rgb(0.25, 0.25, 0.25),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        card_col = card_col.push(debug_block);
    }

    // ── Disconnect button ────────────────────────────────────────────────
    let disconnect_btn = button(
        text("Disconnect").size(12)
            .color(Color::from_rgb(1.0, 0.4, 0.4))
    )
        .on_press(Message::DeviceDisconnect(idx))
        .style(|theme: &iced::Theme, status| button::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.08, 0.08))),
            border: iced::Border {
                color: Color::from_rgb(0.5, 0.15, 0.15),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..button::secondary(theme, status)
        })
        .padding([5, 12]);

    card_col = card_col.push(
        row![Space::new().width(Length::Fill), disconnect_btn]
    );

    container(card_col)
        .width(Length::Fill)
        .padding(16)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.13))),
            border: iced::Border {
                color: Color::from_rgb(0.25, 0.25, 0.25),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn label(s: &str) -> Element<Message> {
    text(format!("{s}:"))
        .size(12)
        .color(Color::from_rgb(0.5, 0.5, 0.5))
        .width(Length::Fixed(70.0))
        .into()
}