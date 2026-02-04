use iced::widget::{column, container, row, slider, text, Column, Image};
use iced::{Element, Subscription, Task};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use windows::core::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;

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
    icon_handle: Option<Arc<iced::widget::image::Handle>>,
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

// Extract icon from executable and save as PNG
unsafe fn extract_icon_to_handle(exe_path: &str) -> Option<iced::widget::image::Handle> {
    let exe_path_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();

    let mut large_icon = HICON::default();
    let result = ExtractIconExW(
        PCWSTR(exe_path_wide.as_ptr()),
        0,
        Some(&mut large_icon),
        None,
        1,
    );

    if result == 0 || large_icon.is_invalid() {
        return None;
    }

    let mut icon_info = ICONINFO::default();
    if !GetIconInfo(large_icon, &mut icon_info).is_ok() {
        DestroyIcon(large_icon);
        return None;
    }

    let mut bm = BITMAP::default();
    GetObjectW(
        icon_info.hbmColor.into(),  // Add .into() here
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    );

    let width = bm.bmWidth as u32;
    let height = bm.bmHeight as u32;

    let hdc = GetDC(Some(HWND::default()));
    let mem_dc = CreateCompatibleDC(Some(hdc));

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let dib = CreateDIBSection(
        Some(mem_dc),
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    ).ok()?;

    let old_bitmap = SelectObject(mem_dc, dib.into());
    DrawIconEx(mem_dc, 0, 0, large_icon, width as i32, height as i32, 0, Some(HBRUSH::default()), DI_NORMAL);

    // Convert BGRA to RGBA in memory
    let pixel_count = (width * height) as usize;
    let src_pixels = std::slice::from_raw_parts(bits as *const u8, pixel_count * 4);
    let mut rgba_data = Vec::with_capacity(pixel_count * 4);

    for i in 0..pixel_count {
        let offset = i * 4;
        let b = src_pixels[offset];
        let g = src_pixels[offset + 1];
        let r = src_pixels[offset + 2];
        let a = src_pixels[offset + 3];

        rgba_data.push(r);
        rgba_data.push(g);
        rgba_data.push(b);
        rgba_data.push(a);
    }

    // Cleanup
    SelectObject(mem_dc, old_bitmap);
    DeleteObject(dib.into());
    DeleteDC(mem_dc);
    ReleaseDC(Some(HWND::default()), hdc);
    DeleteObject(icon_info.hbmColor.into());
    DeleteObject(icon_info.hbmMask.into());
    DestroyIcon(large_icon);

    // Create Iced image handle from bytes in memory
    Some(iced::widget::image::Handle::from_rgba(
        width,
        height,
        rgba_data,
    ))
}

unsafe fn get_process_info(pid: u32) -> Option<(String, String)> {
    if pid == 0 {
        return None;
    }

    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; MAX_PATH as usize];
    let mut size = buffer.len() as u32;
    let mut pwstr = PWSTR(buffer.as_mut_ptr());

    if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, pwstr, &mut size).is_ok() {
        let _ = CloseHandle(handle);
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        let name = full_path
            .split('\\')
            .last()
            .map(|s| s.trim_end_matches(".exe").to_string())?;

        return Some((name, full_path));
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

        // Create icons directory if it doesn't exist
        let icons_dir = std::env::temp_dir().join("volume_mixer_icons");
        let _ = std::fs::create_dir_all(&icons_dir);

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

                                let (display_name, exe_path) = if let Some((name, path)) = get_process_info(process_id) {
                                    (name, Some(path))
                                } else if let Ok(name) = session_control.GetDisplayName() {
                                    let name_str = name.to_string().unwrap_or_default();
                                    if !name_str.is_empty() && !name_str.starts_with("@%") {
                                        (name_str, None)
                                    } else {
                                        (format!("Process {}", process_id), None)
                                    }
                                } else {
                                    (format!("Process {}", process_id), None)
                                };

                                // Extract icon if we have an exe path
                                let icon_handle = if let Some(ref exe) = exe_path {
                                    extract_icon_to_handle(exe).map(Arc::new)
                                } else {
                                    None
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
                                        icon_handle,
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
                        existing.volume = session.volume;
                        if existing.icon_handle.is_none() && session.icon_handle.is_some() {
                            existing.icon_handle = session.icon_handle;
                        }
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

            // Create row with icon and name
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

            content = content.push(
                column![
                    header,
                    slider_widget,
                    text(format!("{}%", (session.volume * 100.0) as i32))
                ]
                    .spacing(5),
            );
        }

        container(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::RefreshSessions)
    }
}

fn main() -> iced::Result {
    iced::application(VolumeApp::new, VolumeApp::update, VolumeApp::view)
        .subscription(VolumeApp::subscription)
        .run()
}