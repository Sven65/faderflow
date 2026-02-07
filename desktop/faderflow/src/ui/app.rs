use crate::audio::{create_backend, AudioBackend, AudioSession, AudioUpdate};
use iced::futures;
use iced::widget::{button, column, container, row, slider, text, Column, Image};
use iced::{Element, Subscription, Task};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct VolumeApp {
    sessions: HashMap<String, AudioSession>,
    receiver: mpsc::Receiver<AudioUpdate>,
    backend: Box<dyn AudioBackend>,
}

#[derive(Debug, Clone)]
pub enum Message {
    VolumeChanged(String, f32),
    ToggleMute(String),
    RefreshSessions,
    SessionsUpdated(Vec<AudioSession>),
    PollReceiver,
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
            Message::RefreshSessions => {
                match self.backend.get_sessions() {
                    Ok(sessions) => Task::done(Message::SessionsUpdated(sessions)),
                    Err(_) => Task::done(Message::SessionsUpdated(Vec::new())),
                }
            }
            Message::SessionsUpdated(sessions) => {
                for session in &sessions {  // Add & here
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
                            existing.icon_handle = session.icon_handle.clone();  // Also add .clone() here
                        }
                    } else {
                        self.sessions.insert(session.id.clone(), session.clone());  // Add .clone() here too
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
                            let entry = last_updates.entry(id.clone()).or_insert((None, None));
                            entry.0 = Some(volume);
                        }
                        AudioUpdate::MuteChanged(ref id, muted) => {
                            let entry = last_updates.entry(id.clone()).or_insert((None, None));
                            entry.1 = Some(muted);
                        }
                        _ => {}
                    }
                }

                for (id, (volume_opt, mute_opt)) in last_updates {
                    if let Some(session) = self.sessions.get_mut(&id) {
                        let ignore_due_to_local = session
                            .last_local_change
                            .map(|t| t.elapsed() < Duration::from_millis(50))
                            .unwrap_or(false);

                        if !ignore_due_to_local {
                            if let Some(volume) = volume_opt {
                                session.volume = volume;
                            }
                            if let Some(muted) = mute_opt {
                                session.is_muted = muted;
                            }
                            session.last_external_change = Some(Instant::now());
                        }
                    }
                }

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut content: Column<Message> = column![].spacing(20).padding(20);

        if self.sessions.is_empty() {
            content = content.push(text("No audio sessions found. Play some audio..."));
        }

        for (id, session) in &self.sessions {
            let slider_widget = slider(0.0..=1.0, session.volume, {
                let id = id.clone();
                move |v| Message::VolumeChanged(id.clone(), v)
            })
                .step(0.01);

            let mute_button = button(text(if session.is_muted { "🔇" } else { "🔊" }))
                .on_press({
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

            content = content.push(column![header, volume_control].spacing(5));
        }

        container(content).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let refresh_timer =
            iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::RefreshSessions);

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