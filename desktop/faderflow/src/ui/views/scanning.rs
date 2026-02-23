use iced::widget::{column, container, progress_bar, row, scrollable, text};

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use iced::{Alignment, Element, Length};

use crate::comms::scanner::SharedPort;
use crate::ui::app::Message;

#[derive(Default)]
pub struct ScanningState {
    pub status: String,
    pub progress: f32,                              // 0.0 – 1.0
    pub log: Vec<LogEntry>,
    pub found_devices: Vec<(String, SharedPort, [u8; 16], (u8, u8), Arc<AtomicBool>)>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub text: String,
    pub kind: LogKind,
}

#[derive(Debug, Clone)]
pub enum LogKind {
    Info,
    Success,
    Failure,
}

impl ScanningState {
    pub fn push_log(&mut self, text: impl Into<String>, kind: LogKind) {
        self.log.push(LogEntry { text: text.into(), kind });
    }
}

pub fn view(state: &ScanningState) -> Element<Message> {
    let title = text("Scanning for FaderFlow devices")
        .size(22);

    let status = text(&state.status)
        .size(14);

    let bar = progress_bar(0.0..=1.0, state.progress);

    let log_entries: Vec<Element<Message>> = state.log.iter().map(|entry| {
        let (prefix, color) = match entry.kind {
            LogKind::Info    => ("  •  ", iced::Color::from_rgb(0.7, 0.7, 0.7)),
            LogKind::Success => ("  ✓  ", iced::Color::from_rgb(0.2, 0.9, 0.4)),
            LogKind::Failure => ("  ✗  ", iced::Color::from_rgb(0.9, 0.3, 0.3)),
        };
        row![
            text(prefix).color(color).size(13),
            text(&entry.text).color(color).size(13),
        ]
            .into()
    }).collect();

    let log_scroll = scrollable(
        column(log_entries).spacing(2)
    )
        .height(Length::Fixed(180.0));

    let content = column![
        title,
        bar,
        status,
        log_scroll,
    ]
        .spacing(12)
        .spacing(0)
        .align_x(Alignment::Center)
        .width(Length::Fixed(480.0));

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}