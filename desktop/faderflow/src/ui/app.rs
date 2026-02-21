use crate::audio::{create_backend, AudioBackend, AudioSession, AudioUpdate};
use crate::ui::views;
use iced::futures;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Subscription, Task};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct VolumeApp {
    pub sessions: HashMap<String, AudioSession>,
    receiver: mpsc::Receiver<AudioUpdate>,
    backend: Box<dyn AudioBackend>,
    current_view: View,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Sessions,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    VolumeChanged(String, f32),
    ToggleMute(String),
    RefreshSessions,
    SessionsUpdated(Vec<AudioSession>),
    PollReceiver,
    ShowSessions,
    ShowSettings,
    ShowAbout,
}

impl VolumeApp {
    pub fn new() -> (Self, Task<Message>) {
        let (sender, receiver) = mpsc::channel();
        let mut backend = create_backend();

        let _ = backend.initialize();
        let _ = backend.start_listening(sender);

        (
            Self {
                sessions: HashMap::new(),
                receiver,
                backend,
                current_view: View::Sessions,
            },
            Task::done(Message::RefreshSessions),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::VolumeChanged(id, volume) => {
                if let Some(session) = self.sessions.get_mut(&id) {
                    session.volume = volume;
                    session.last_local_change = Some(Instant::now());
                    let _ = self.backend.set_volume(&id, volume);
                }
                Task::none()
            }
            Message::ToggleMute(id) => {
                if let Some(session) = self.sessions.get_mut(&id) {
                    session.is_muted = !session.is_muted;
                    session.last_local_change = Some(Instant::now());
                    let _ = self.backend.set_mute(&id, session.is_muted);
                }
                Task::none()
            }
            Message::RefreshSessions => match self.backend.get_sessions() {
                Ok(sessions) => Task::done(Message::SessionsUpdated(sessions)),
                Err(_) => Task::done(Message::SessionsUpdated(Vec::new())),
            },
            Message::SessionsUpdated(sessions) => {
                for session in &sessions {
                    if let Some(existing) = self.sessions.get_mut(&session.id) {
                        let should_update = existing
                            .last_local_change
                            .map(|t| t.elapsed() > Duration::from_millis(100))
                            .unwrap_or(true)
                            && existing
                            .last_external_change
                            .map(|t| t.elapsed() > Duration::from_millis(500))
                            .unwrap_or(true);

                        if should_update {
                            existing.volume = session.volume;
                            existing.is_muted = session.is_muted;
                        }

                        if existing.icon_handle.is_none() && session.icon_handle.is_some() {
                            existing.icon_handle = session.icon_handle.clone();
                        }
                    } else {
                        self.sessions.insert(session.id.clone(), session.clone());
                    }
                }

                let session_ids: Vec<String> = self.sessions.keys().cloned().collect();
                for id in session_ids {
                    if !sessions.iter().any(|s| s.id == id) {
                        self.sessions.remove(&id);
                    }
                }

                Task::none()
            }
            Message::PollReceiver => {
                let mut updates = Vec::new();
                while let Ok(update) = self.receiver.try_recv() {
                    updates.push(update);
                }

                let mut last_updates: HashMap<String, (Option<f32>, Option<bool>)> =
                    HashMap::new();

                for update in updates {
                    match update {
                        AudioUpdate::VolumeChanged(ref id, volume) => {
                            last_updates.entry(id.clone()).or_insert((None, None)).0 =
                                Some(volume);
                        }
                        AudioUpdate::MuteChanged(ref id, muted) => {
                            last_updates.entry(id.clone()).or_insert((None, None)).1 =
                                Some(muted);
                        }
                        _ => {}
                    }
                }

                for (id, (volume_opt, mute_opt)) in last_updates {
                    if let Some(session) = self.sessions.get_mut(&id) {
                        let ignore = session
                            .last_local_change
                            .map(|t| t.elapsed() < Duration::from_millis(50))
                            .unwrap_or(false);

                        if !ignore {
                            if let Some(v) = volume_opt {
                                session.volume = v;
                            }
                            if let Some(m) = mute_opt {
                                session.is_muted = m;
                            }
                            session.last_external_change = Some(Instant::now());
                        }
                    }
                }

                Task::none()
            }
            Message::ShowSessions => {
                self.current_view = View::Sessions;
                Task::none()
            }
            Message::ShowSettings => {
                self.current_view = View::Settings;
                Task::none()
            }
            Message::ShowAbout => {
                self.current_view = View::About;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let sidebar = container(
            column![
                self.sidebar_button("🎵 Sessions", View::Sessions),
                self.sidebar_button("⚙️ Settings", View::Settings),
                self.sidebar_button("ℹ️ About", View::About),
            ]
                .spacing(10)
                .padding(20),
        )
            .width(200)
            .height(iced::Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.15, 0.15, 0.15,
                ))),
                ..Default::default()
            });

        let main_content = container(match self.current_view {
            View::Sessions => views::sessions::view(&self.sessions),
            View::Settings => views::settings::view(),
            View::About => views::about::view(),
        })
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .padding(20);

        row![sidebar, main_content].into()
    }

    fn sidebar_button<'a>(&'a self, label: &'a str, view: View) -> Element<'a, Message> {
        let is_active = self.current_view == view;
        let btn = button(text(label)).width(iced::Length::Fill).padding(10);

        let btn = if is_active {
            btn.style(|theme: &iced::Theme, status| button::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.3, 0.3, 0.3,
                ))),
                text_color: iced::Color::WHITE,
                ..button::primary(theme, status)
            })
        } else {
            btn
        };

        match view {
            View::Sessions => btn.on_press(Message::ShowSessions).into(),
            View::Settings => btn.on_press(Message::ShowSettings).into(),
            View::About => btn.on_press(Message::ShowAbout).into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let refresh_timer =
            iced::time::every(Duration::from_secs(2)).map(|_| Message::RefreshSessions);

        let receiver_subscription = iced::Subscription::run(|| {
            use futures::stream::StreamExt;
            use iced::stream;

            stream::channel(
                100,
                |mut output: futures::channel::mpsc::Sender<Message>| async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                        let _ = output.try_send(Message::PollReceiver);
                    }
                },
            )
        });

        Subscription::batch(vec![refresh_timer, receiver_subscription])
    }
}