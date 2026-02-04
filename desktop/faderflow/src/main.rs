use iced::widget::{column, container, slider, text, Column};
use iced::{Element, Subscription, Task};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Threading::*;
use windows::Win32::Foundation::*;

// Store raw pointers instead of COM interfaces
static AUDIO_CONTROLS: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();
static COM_INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();

fn get_controls() -> &'static Mutex<HashMap<String, usize>> {
    AUDIO_CONTROLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_com_initialized() -> &'static Mutex<bool> {
    COM_INITIALIZED.get_or_init(|| Mutex::new(false))
}

#[derive(Debug, Clone)]
struct AudioSession {
    display_name: String,
    volume: f32,
    process_id: u32,
}

#[derive(Default)]
struct VolumeApp {
    sessions: HashMap<String, AudioSession>,
}

#[derive(Debug, Clone)]
enum Message {
    VolumeChanged(String, f32),
    RefreshSessions,
    SessionsUpdated(Vec<AudioSession>),
}

fn main() -> iced::Result {
    iced::application(VolumeApp::new, VolumeApp::update, VolumeApp::view)
        .subscription(VolumeApp::subscription)
        .run()
}

impl VolumeApp {
    fn new() -> (Self, Task<Message>) {
        (
            VolumeApp {
                sessions: HashMap::new(),
            },
            Task::done(Message::RefreshSessions),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::VolumeChanged(name, volume) => {
                if let Some(session) = self.sessions.get_mut(&name) {
                    session.volume = volume;

                    // Update Windows volume using raw pointer
                    if let Ok(controls) = get_controls().lock() {
                        if let Some(&ptr) = controls.get(&name) {
                            unsafe {
                                let control = ISimpleAudioVolume::from_raw(ptr as *mut _);
                                let _ = control.SetMasterVolume(volume, std::ptr::null());
                                std::mem::forget(control);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::RefreshSessions => Task::perform(get_audio_sessions(), Message::SessionsUpdated),
            Message::SessionsUpdated(sessions) => {
                for session in sessions {
                    if let Some(existing) = self.sessions.get_mut(&session.display_name) {
                        // Always update volume from Windows for two-way sync
                        existing.volume = session.volume;
                    } else {
                        self.sessions.insert(session.display_name.clone(), session);
                    }
                }

                let session_names: Vec<String> = self.sessions.keys().cloned().collect();
                if let Ok(controls) = get_controls().lock() {
                    for name in session_names {
                        if !controls.contains_key(&name) {
                            self.sessions.remove(&name);
                        }
                    }
                }

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let mut content: Column<Message> = column![].spacing(20).padding(20);

        if self.sessions.is_empty() {
            content = content.push(text("No audio sessions found. Play some audio..."));
        }

        for (name, session) in &self.sessions {
            let slider_widget = slider(0.0..=1.0, session.volume, {
                let name = name.clone();
                move |v| Message::VolumeChanged(name.clone(), v)
            })
                .step(0.01);

            content = content.push(
                column![
                    text(&session.display_name),
                    slider_widget,
                    text(format!("{}%", (session.volume * 100.0) as i32))
                ]
                    .spacing(5),
            );
        }

        container(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Poll every 100ms for more responsive updates
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::RefreshSessions)
    }
}

unsafe fn get_process_name(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }

    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

    let mut buffer = [0u16; MAX_PATH as usize];
    let mut size = buffer.len() as u32;

    let mut pwstr = PWSTR(buffer.as_mut_ptr());

    if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, pwstr, &mut size).is_ok() {
        let _ = CloseHandle(handle);
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        return path.split('\\').last().map(|s| {
            s.trim_end_matches(".exe").to_string()
        });
    }

    let _ = CloseHandle(handle);
    None
}

async fn get_audio_sessions() -> Vec<AudioSession> {
    unsafe {
        if let Ok(mut initialized) = is_com_initialized().lock() {
            if !*initialized {
                if CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() {
                    *initialized = true;
                } else {
                    return Vec::new();
                }
            }
        }

        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let session_manager: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
            Ok(sm) => sm,
            Err(_) => return Vec::new(),
        };

        let session_enumerator = match session_manager.GetSessionEnumerator() {
            Ok(se) => se,
            Err(_) => return Vec::new(),
        };

        let count = match session_enumerator.GetCount() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut sessions = Vec::new();
        let mut new_controls = HashMap::new();

        for i in 0..count {
            if let Ok(session_control) = session_enumerator.GetSession(i) {
                if let Ok(state) = session_control.GetState() {
                    if state == AudioSessionStateActive || state == AudioSessionStateInactive {
                        if let Ok(session_control2) = session_control.cast::<IAudioSessionControl2>() {
                            if let Ok(volume_control) = session_control.cast::<ISimpleAudioVolume>() {
                                let process_id = session_control2.GetProcessId().unwrap_or(0);

                                if process_id == 0 {
                                    continue;
                                }

                                let display_name = if let Ok(name) = session_control.GetDisplayName() {
                                    let name_str = name.to_string().unwrap_or_default();
                                    if !name_str.is_empty() && !name_str.starts_with("@%") {
                                        name_str
                                    } else {
                                        get_process_name(process_id)
                                            .unwrap_or_else(|| format!("Process {}", process_id))
                                    }
                                } else {
                                    get_process_name(process_id)
                                        .unwrap_or_else(|| format!("Process {}", process_id))
                                };

                                if let Ok(volume) = volume_control.GetMasterVolume() {
                                    if let Ok(controls_map) = get_controls().lock() {
                                        if let Some(&existing_ptr) = controls_map.get(&display_name) {
                                            new_controls.insert(display_name.clone(), existing_ptr);
                                            drop(volume_control);
                                        } else {
                                            let ptr = volume_control.as_raw() as usize;
                                            new_controls.insert(display_name.clone(), ptr);
                                            std::mem::forget(volume_control);
                                        }
                                    }

                                    sessions.push(AudioSession {
                                        display_name,
                                        volume,
                                        process_id,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Ok(mut controls_map) = get_controls().lock() {
            for (name, ptr) in controls_map.drain() {
                if !new_controls.contains_key(&name) {
                    let control = ISimpleAudioVolume::from_raw(ptr as *mut _);
                    drop(control);
                }
            }

            *controls_map = new_controls;
        }

        sessions
    }
}