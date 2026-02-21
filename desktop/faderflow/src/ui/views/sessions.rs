use crate::audio::AudioSession;
use crate::ui::app::Message;
use iced::widget::{button, column, container, row, slider, text, Column, Image};
use iced::Element;
use std::collections::HashMap;

pub fn view<'a>(sessions: &'a HashMap<String, AudioSession>) -> Element<'a, Message> {
    let mut content: Column<Message> = column![text("Audio Sessions").size(24)].spacing(20);

    if sessions.is_empty() {
        content = content.push(text("No audio sessions found. Play some audio..."));
    }

    for (id, session) in sessions {
        let slider_widget = slider(0.0..=1.0, session.volume, {
            let id = id.clone();
            move |v| Message::VolumeChanged(id.clone(), v)
        })
            .step(0.01);

        let mute_button = button(text(if session.is_muted { "🔇" } else { "🔊" })).on_press({
            let id = id.clone();
            Message::ToggleMute(id)
        });

        let header = if let Some(icon_handle) = &session.icon_handle {
            row![
                Image::new(icon_handle.as_ref().clone())
                    .width(24)
                    .height(24),
                text(&session.display_name)
            ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
        } else {
            row![text(&session.display_name)]
        };

        let volume_control = row![
            slider_widget,
            text(format!("{}%", (session.volume * 100.0) as i32)).width(50),
            mute_button
        ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

        content = content.push(
            container(column![header, volume_control].spacing(5))
                .padding(10)
                .style(|_theme: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.1, 0.1, 0.1,
                    ))),
                    border: iced::Border {
                        radius: 5.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    column![content].into()
}