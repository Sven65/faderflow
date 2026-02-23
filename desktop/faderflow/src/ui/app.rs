use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use iced::futures;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Subscription, Task};

use crate::audio::{create_backend, AudioBackend, AudioSession, AudioUpdate};
use crate::comms::scanner::{self, ScanEvent, RESCAN_DELAY_SECS};
use crate::comms::device_info::{DeviceInfo, DeviceStatus};
use crate::utils::config::{
    load_device_renames, load_device_assignments,
    save_device_renames, save_device_assignments,
    send_app_name, send_volume,
};
use crate::ui::views;
use crate::ui::views::no_devices::NoDevicesReason;
use crate::ui::views::scanning::{LogKind, ScanningState};

// ── App screens ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum View {
    Sessions,
    Settings,
    About,
    Devices,
}

pub enum AppScreen {
    Scanning(ScanningState),
    NoDevices(NoDevicesReason),
    Ready(ReadyState),
}

pub struct ReadyState {
    pub devices: Vec<DeviceInfo>,
    pub sessions: HashMap<String, AudioSession>,
    pub current_view: View,
    pub rename_drafts: Vec<String>,
    pub debug_open: Vec<bool>,
    pub output_devices: Vec<String>,
    pub current_output: Option<String>,
}

// ── App ──────────────────────────────────────────────────────────────────────

pub struct VolumeApp {
    screen: AppScreen,
    audio_rx: mpsc::Receiver<AudioUpdate>,
    backend: Box<dyn AudioBackend>,
    scan_rx: Option<Arc<Mutex<mpsc::Receiver<ScanEvent>>>>,
    watchdog_tx: mpsc::Sender<ScanEvent>,
    watchdog_rx: Arc<Mutex<mpsc::Receiver<ScanEvent>>>,
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ShowSessions,
    ShowSettings,
    ShowAbout,
    ShowDevices,
    VolumeChanged(String, f32),
    ToggleMute(String),
    RefreshSessions,
    SessionsUpdated(Vec<AudioSession>),
    PollAudioReceiver,
    StartScan,
    ScanTick,
    WatchdogTick,
    RetryTick,
    DeviceRenameDraft(usize, String),
    DeviceRenameCommit(usize),
    DeviceToggleDebug(usize),
    DeviceDisconnect(usize),
    DeviceChannelAssign(usize, usize, String), // device_idx, channel, session_name
    DeviceSync(usize),
    SelectOutput(String),                      // ← was missing from enum
}

// ── Constructor ──────────────────────────────────────────────────────────────

impl VolumeApp {
    pub fn new() -> (Self, Task<Message>) {
        let (audio_tx, audio_rx) = mpsc::channel();
        let mut backend = create_backend();
        let _ = backend.initialize();
        let _ = backend.start_listening(audio_tx);

        let (watchdog_tx, watchdog_rx) = mpsc::channel();

        let app = Self {
            screen: AppScreen::Scanning(ScanningState::default()),
            audio_rx,
            backend,
            scan_rx: None,
            watchdog_tx,
            watchdog_rx: Arc::new(Mutex::new(watchdog_rx)),
        };

        (app, Task::done(Message::StartScan))
    }
}

// ── Update ───────────────────────────────────────────────────────────────────

impl VolumeApp {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Navigation ────────────────────────────────────────────────
            Message::ShowSessions => {
                if let AppScreen::Ready(s) = &mut self.screen { s.current_view = View::Sessions; }
                Task::none()
            }
            Message::ShowSettings => {
                if let AppScreen::Ready(s) = &mut self.screen { s.current_view = View::Settings; }
                Task::none()
            }
            Message::ShowAbout => {
                if let AppScreen::Ready(s) = &mut self.screen { s.current_view = View::About; }
                Task::none()
            }
            Message::ShowDevices => {
                if let AppScreen::Ready(s) = &mut self.screen { s.current_view = View::Devices; }
                Task::none()
            }

            // ── Audio ─────────────────────────────────────────────────────
            Message::VolumeChanged(id, volume) => {
                if let AppScreen::Ready(s) = &mut self.screen {
                    if let Some(session) = s.sessions.get_mut(&id) {
                        session.volume = volume;
                        session.last_local_change = Some(Instant::now());
                        let _ = self.backend.set_volume(&id, volume);
                    }
                }
                Task::none()
            }
            Message::ToggleMute(id) => {
                if let AppScreen::Ready(s) = &mut self.screen {
                    if let Some(session) = s.sessions.get_mut(&id) {
                        session.is_muted = !session.is_muted;
                        session.last_local_change = Some(Instant::now());
                        let _ = self.backend.set_mute(&id, session.is_muted);
                    }
                }
                Task::none()
            }
            Message::RefreshSessions => match self.backend.get_sessions() {
                Ok(sessions) => Task::done(Message::SessionsUpdated(sessions)),
                Err(_) => Task::done(Message::SessionsUpdated(Vec::new())),
            },
            Message::SessionsUpdated(sessions) => {
                if let AppScreen::Ready(s) = &mut self.screen {
                    for session in &sessions {
                        if let Some(existing) = s.sessions.get_mut(&session.id) {
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
                            s.sessions.insert(session.id.clone(), session.clone());
                        }
                    }
                    s.sessions.retain(|id, _| sessions.iter().any(|s| &s.id == id));
                }
                Task::none()
            }
            Message::PollAudioReceiver => {
                if let AppScreen::Ready(s) = &mut self.screen {
                    let mut last_updates: HashMap<String, (Option<f32>, Option<bool>)> =
                        HashMap::new();
                    while let Ok(update) = self.audio_rx.try_recv() {
                        match update {
                            AudioUpdate::VolumeChanged(ref id, v) => {
                                last_updates.entry(id.clone()).or_default().0 = Some(v);
                            }
                            AudioUpdate::MuteChanged(ref id, m) => {
                                last_updates.entry(id.clone()).or_default().1 = Some(m);
                            }
                            AudioUpdate::DefaultDeviceChanged(name) => {
                                s.current_output = Some(name);
                                return Task::done(Message::RefreshSessions);
                            }
                            AudioUpdate::SessionAdded(_) | AudioUpdate::SessionRemoved(_) => {
                                return Task::done(Message::RefreshSessions);
                            }
                        }
                    }
                    for (id, (vol, mute)) in last_updates {
                        if let Some(session) = s.sessions.get_mut(&id) {
                            let ignore = session
                                .last_local_change
                                .map(|t| t.elapsed() < Duration::from_millis(50))
                                .unwrap_or(false);
                            if !ignore {
                                if let Some(v) = vol  { session.volume   = v; }
                                if let Some(m) = mute { session.is_muted = m; }
                                session.last_external_change = Some(Instant::now());
                            }
                        }
                    }
                }
                Task::none()
            }

            // ── Scanning ──────────────────────────────────────────────────
            Message::StartScan => {
                // Drop all existing devices so their ports are freed before scanning
                if let AppScreen::Ready(state) = &mut self.screen {
                    for dev in state.devices.drain(..) {
                        dev.cancel_watchdog();
                        drop(dev.port);
                    }
                }
                self.screen = AppScreen::Scanning(ScanningState::default());
                if let Ok(rx) = self.watchdog_rx.lock() {
                    while rx.try_recv().is_ok() {}
                }
                let rx = scanner::start_scan_delayed(500);
                self.scan_rx = Some(Arc::new(Mutex::new(rx)));
                Task::none()
            }
            Message::ScanTick => {
                let events: Vec<ScanEvent> = self.scan_rx
                    .as_ref()
                    .map(|rx| {
                        let rx = rx.lock().unwrap();
                        std::iter::from_fn(|| rx.try_recv().ok()).collect()
                    })
                    .unwrap_or_default();
                for ev in events {
                    self.handle_scan_event(ev);
                }
                Task::none()
            }
            Message::WatchdogTick => {
                let already_lost = matches!(self.screen, AppScreen::NoDevices(_));
                let events: Vec<ScanEvent> = {
                    let rx = self.watchdog_rx.lock().unwrap();
                    std::iter::from_fn(|| rx.try_recv().ok()).collect()
                };
                if !already_lost {
                    for ev in events {
                        if let ScanEvent::DeviceLost { port_name } = ev {
                            self.screen = AppScreen::NoDevices(NoDevicesReason::Lost {
                                port_name,
                                retry_in_secs: RESCAN_DELAY_SECS,
                            });
                            self.scan_rx = None;
                        }
                    }
                }
                Task::none()
            }
            Message::RetryTick => {
                if let AppScreen::NoDevices(NoDevicesReason::Lost { retry_in_secs, .. }) =
                    &mut self.screen
                {
                    if *retry_in_secs > 1 {
                        *retry_in_secs -= 1;
                    } else {
                        return Task::done(Message::StartScan);
                    }
                }
                Task::none()
            }

            // ── Device management ─────────────────────────────────────────
            Message::DeviceRenameDraft(idx, s) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if let Some(draft) = state.rename_drafts.get_mut(idx) {
                        *draft = s;
                    }
                }
                Task::none()
            }
            Message::DeviceRenameCommit(idx) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if let (Some(dev), Some(draft)) = (
                        state.devices.get_mut(idx),
                        state.rename_drafts.get(idx),
                    ) {
                        let name = draft.trim().to_string();
                        dev.rename = if name.is_empty() { None } else { Some(name) };
                        save_device_renames(&state.devices);
                    }
                }
                Task::none()
            }
            Message::DeviceToggleDebug(idx) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if let Some(open) = state.debug_open.get_mut(idx) {
                        *open = !*open;
                    }
                }
                Task::none()
            }
            Message::SelectOutput(name) => {
                if let AppScreen::Ready(s) = &mut self.screen {
                    s.current_output = Some(name);
                }
                Task::done(Message::RefreshSessions)
            }
            Message::DeviceChannelAssign(dev_idx, ch, session) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if let Some(dev) = state.devices.get_mut(dev_idx) {
                        if ch < 5 {
                            dev.channel_assignments[ch] = session;
                            save_device_assignments(&state.devices);
                        }
                    }
                }
                Task::none()
            }
            Message::DeviceSync(dev_idx) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if let Some(dev) = state.devices.get_mut(dev_idx) {
                        let port = Arc::clone(&dev.port);
                        for (ch, session_name) in dev.channel_assignments.iter().enumerate() {
                            if let Ok(mut p) = port.lock() {
                                send_app_name(&mut **p, ch as u8, session_name);
                                let vol = if session_name.is_empty() {
                                    0
                                } else {
                                    state.sessions.get(session_name)
                                        .map(|s| (s.volume * 100.0) as u8)
                                        .unwrap_or(0)
                                };
                                send_volume(&mut **p, ch as u8, vol);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::DeviceDisconnect(idx) => {
                if let AppScreen::Ready(state) = &mut self.screen {
                    if idx < state.devices.len() {
                        let dev = state.devices.remove(idx);
                        dev.cancel_watchdog();
                        drop(dev.port);
                        state.rename_drafts.remove(idx);
                        state.debug_open.remove(idx);
                    }
                    if state.devices.is_empty() {
                        self.screen = AppScreen::NoDevices(NoDevicesReason::NoneFound);
                    }
                }
                Task::none()
            }
        }
    }

    fn handle_scan_event(&mut self, ev: ScanEvent) {
        match ev {
            ScanEvent::Started { total_ports } => {
                if let AppScreen::Scanning(s) = &mut self.screen {
                    s.status = format!("Found {total_ports} port(s) to check…");
                    s.push_log(format!("Scanning {total_ports} port(s)"), LogKind::Info);
                }
            }
            ScanEvent::CheckingPort { name, index, total } => {
                if let AppScreen::Scanning(s) = &mut self.screen {
                    s.status = format!("Checking {name}…");
                    s.progress = (index as f32 - 1.0) / total as f32 * 0.9;
                    s.push_log(format!("Checking {name}…"), LogKind::Info);
                }
            }
            ScanEvent::PortFailed { name, reason } => {
                if let AppScreen::Scanning(s) = &mut self.screen {
                    s.push_log(format!("{name}: {reason}"), LogKind::Failure);
                }
            }
            ScanEvent::DeviceFound { port_name, port, uuid, version } => {
                if let AppScreen::Scanning(s) = &mut self.screen {
                    s.push_log(format!("{port_name}: FaderFlow ✓"), LogKind::Success);
                    let watchdog_cancel = scanner::start_watchdog(
                        port_name.clone(),
                        Arc::clone(&port),
                        self.watchdog_tx.clone(),
                    );
                    s.found_devices.push((port_name, port, uuid, version, watchdog_cancel));
                }
            }
            ScanEvent::ScanComplete { found } => {
                self.scan_rx = None;

                if found == 0 {
                    self.screen = AppScreen::NoDevices(NoDevicesReason::NoneFound);
                    return;
                }

                let raw_devices = if let AppScreen::Scanning(s) = &mut self.screen {
                    s.progress = 1.0;
                    std::mem::take(&mut s.found_devices)
                } else {
                    vec![]
                };

                let saved_renames = load_device_renames();
                let saved_assignments = load_device_assignments();
                let n = raw_devices.len();
                let devices: Vec<DeviceInfo> = raw_devices
                    .into_iter()
                    .map(|(port_name, port, uuid, version, watchdog_cancel)| {
                        let uuid_str = DeviceInfo::uuid_str(&uuid);
                        let rename = saved_renames.get(&uuid_str).cloned();
                        let channel_assignments = saved_assignments
                            .get(&uuid_str)
                            .cloned()
                            .unwrap_or_default();
                        DeviceInfo {
                            port_name, port, uuid, version, rename,
                            status: DeviceStatus::Connected,
                            watchdog_cancel,
                            channel_assignments,
                        }
                    })
                    .collect();

                self.screen = AppScreen::Ready(ReadyState {
                    devices,
                    sessions: HashMap::new(),
                    current_view: View::Sessions,
                    rename_drafts: vec![String::new(); n],
                    debug_open: vec![false; n],
                    output_devices: self.backend.get_output_devices().unwrap_or_default(),
                    current_output: self.backend.get_default_output_device(),
                });
            }
            ScanEvent::ScanFailed(reason) => {
                eprintln!("Scan failed: {reason}");
                self.scan_rx = None;
                self.screen = AppScreen::NoDevices(NoDevicesReason::NoneFound);
            }
            ScanEvent::DeviceLost { port_name } => {
                self.screen = AppScreen::NoDevices(NoDevicesReason::Lost {
                    port_name,
                    retry_in_secs: RESCAN_DELAY_SECS,
                });
            }
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

impl VolumeApp {
    pub fn view(&self) -> Element<Message> {
        match &self.screen {
            AppScreen::Scanning(s)  => views::scanning::view(s),
            AppScreen::NoDevices(r) => views::no_devices::view(r),
            AppScreen::Ready(s)     => self.ready_view(s),
        }
    }

    fn ready_view<'a>(&'a self, state: &'a ReadyState) -> Element<'a, Message> {
        let sidebar = container(
            column![
                self.sidebar_button("🎵 Sessions", View::Sessions, &state.current_view),
                self.sidebar_button("⚙️ Settings", View::Settings, &state.current_view),
                self.sidebar_button("🔌 Devices",  View::Devices,  &state.current_view),
                self.sidebar_button("ℹ️ About",    View::About,    &state.current_view),
            ]
                .spacing(10)
                .padding(20),
        )
            .width(200)
            .height(iced::Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(0.15, 0.15, 0.15))),
                ..Default::default()
            });

        let main_content = container(match state.current_view {
            View::Sessions => views::sessions::view(&state.sessions),
            View::Settings => views::settings::view(),
            View::About    => views::about::view(),
            View::Devices  => {
                let session_names: Vec<String> = state.sessions.keys().cloned().collect();
                views::devices::view(
                    &state.devices,
                    &state.rename_drafts,
                    &state.debug_open,
                    session_names,
                    &state.output_devices,
                    state.current_output.clone(),
                )
            }
        })
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .padding(20);

        row![sidebar, main_content].into()
    }

    fn sidebar_button<'a>(
        &'a self,
        label: &'a str,
        view: View,
        current: &View,
    ) -> Element<'a, Message> {
        let is_active = current == &view;
        let btn = button(text(label)).width(iced::Length::Fill).padding(10);
        let btn = if is_active {
            btn.style(|theme: &iced::Theme, status| button::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(0.3, 0.3, 0.3))),
                text_color: iced::Color::WHITE,
                ..button::primary(theme, status)
            })
        } else {
            btn
        };
        match view {
            View::Sessions => btn.on_press(Message::ShowSessions).into(),
            View::Settings => btn.on_press(Message::ShowSettings).into(),
            View::About    => btn.on_press(Message::ShowAbout).into(),
            View::Devices  => btn.on_press(Message::ShowDevices).into(),
        }
    }
}

// ── Subscriptions ────────────────────────────────────────────────────────────

impl VolumeApp {
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        match &self.screen {
            AppScreen::Scanning(_) => {
                subs.push(
                    iced::time::every(Duration::from_millis(50))
                        .map(|_| Message::ScanTick),
                );
            }
            AppScreen::Ready(_) => {
                subs.push(
                    Subscription::run(|| {
                        use iced::stream;
                        stream::channel(
                            100,
                            |mut output: futures::channel::mpsc::Sender<Message>| async move {
                                loop {
                                    tokio::time::sleep(Duration::from_millis(16)).await;
                                    let _ = output.try_send(Message::PollAudioReceiver);
                                }
                            },
                        )
                    })
                );
                subs.push(
                    iced::time::every(Duration::from_secs(2))
                        .map(|_| Message::RefreshSessions),
                );
                subs.push(
                    iced::time::every(Duration::from_millis(500))
                        .map(|_| Message::WatchdogTick),
                );
            }
            AppScreen::NoDevices(NoDevicesReason::Lost { .. }) => {
                subs.push(
                    iced::time::every(Duration::from_secs(1))
                        .map(|_| Message::RetryTick),
                );
            }
            AppScreen::NoDevices(NoDevicesReason::NoneFound) => {}
        }

        Subscription::batch(subs)
    }
}