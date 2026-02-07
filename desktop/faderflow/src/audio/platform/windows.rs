// src/audio/platform/windows.rs
#[cfg(target_os = "windows")]
use crate::audio::{AudioBackend, AudioSession, AudioUpdate};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use windows::core::GUID;
use windows::core::implement;
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;
use windows::Win32::Foundation::*;
use windows::core::PWSTR;
use windows::core::BOOL;
use windows::core::Interface;

static AUDIO_CONTROLS: OnceLock<Mutex<HashMap<String, AudioControlData>>> = OnceLock::new();
static COM_INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();
static APP_STATE: OnceLock<Mutex<Option<AppStateHandle>>> = OnceLock::new();
static APP_CONTEXT_GUID: GUID = GUID::from_u128(0x12345678_1234_1234_1234_123456789abc);

#[derive(Clone)]
struct AudioControlData {
    volume_control: usize,
    session_control: usize,
    callback: usize,
}

#[derive(Clone)]
struct AppStateHandle {
    sender: mpsc::Sender<AudioUpdate>,
}

fn get_controls() -> &'static Mutex<HashMap<String, AudioControlData>> {
    AUDIO_CONTROLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_com_initialized() -> &'static Mutex<bool> {
    COM_INITIALIZED.get_or_init(|| Mutex::new(false))
}

fn get_app_state() -> &'static Mutex<Option<AppStateHandle>> {
    APP_STATE.get_or_init(|| Mutex::new(None))
}

#[implement(IAudioSessionEvents)]
struct AudioSessionCallback {
    session_name: String,
}

impl IAudioSessionEvents_Impl for AudioSessionCallback_Impl {
    fn OnDisplayNameChanged(
        &self,
        _newdisplayname: &PCWSTR,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnIconPathChanged(
        &self,
        _newiconpath: &PCWSTR,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnSimpleVolumeChanged(
        &self,
        newvolume: f32,
        newmute: BOOL,
        eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        unsafe {
            if !eventcontext.is_null() && *eventcontext == APP_CONTEXT_GUID {
                return Ok(());
            }
        }

        if let Ok(state) = get_app_state().lock() {
            if let Some(ref handle) = *state {
                let _ = handle.sender.send(AudioUpdate::VolumeChanged(
                    self.session_name.clone(),
                    newvolume,
                ));
                let _ = handle.sender.send(AudioUpdate::MuteChanged(
                    self.session_name.clone(),
                    newmute.as_bool(),
                ));
            }
        }
        Ok(())
    }

    fn OnChannelVolumeChanged(
        &self,
        _channelcount: u32,
        _newchannelvolumearray: *const f32,
        _changedchannel: u32,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnGroupingParamChanged(
        &self,
        _newgroupingparam: *const GUID,
        _eventcontext: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnStateChanged(&self, _newstate: AudioSessionState) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnSessionDisconnected(
        &self,
        _disconnectreason: AudioSessionDisconnectReason,
    ) -> windows::core::Result<()> {
        Ok(())
    }
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

pub struct WindowsAudioBackend;

impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self
    }
}

impl AudioBackend for WindowsAudioBackend {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            if let Ok(mut initialized) = is_com_initialized().lock() {
                if !*initialized {
                    CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;  // Change to APARTMENTTHREADED
                    *initialized = true;
                }
            }
        }
        Ok(())
    }

    fn get_sessions(&self) -> Result<Vec<AudioSession>, Box<dyn std::error::Error>> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(
                &MMDeviceEnumerator,
                None,
                CLSCTX_ALL,
            )?;

            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let session_manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let session_enumerator = session_manager.GetSessionEnumerator()?;
            let count = session_enumerator.GetCount()?;

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

                                    let icon_handle = if let Some(ref exe) = exe_path {
                                        crate::utils::icon::extract_icon_to_handle(exe).map(Arc::new)
                                    } else {
                                        None
                                    };

                                    if let Ok(volume) = volume_control.GetMasterVolume() {
                                        let is_muted = volume_control.GetMute()
                                            .map(|m| m.as_bool())
                                            .unwrap_or(false);

                                        if let Ok(controls_map) = get_controls().lock() {
                                            if let Some(existing) = controls_map.get(&display_name) {
                                                new_controls.insert(display_name.clone(), existing.clone());
                                                drop(volume_control);
                                                drop(session_control);
                                            } else {
                                                let callback: IAudioSessionEvents = AudioSessionCallback {
                                                    session_name: display_name.clone(),
                                                }.into();

                                                if session_control.RegisterAudioSessionNotification(&callback).is_ok() {
                                                    let control_data = AudioControlData {
                                                        volume_control: volume_control.as_raw() as usize,
                                                        session_control: session_control.as_raw() as usize,
                                                        callback: callback.as_raw() as usize,
                                                    };

                                                    new_controls.insert(display_name.clone(), control_data);

                                                    std::mem::forget(volume_control);
                                                    std::mem::forget(session_control);
                                                    std::mem::forget(callback);
                                                }
                                            }
                                        }

                                        let mut session = AudioSession::new(
                                            display_name.clone(),
                                            display_name,
                                            volume,
                                            is_muted,
                                            process_id,
                                        );
                                        session.icon_handle = icon_handle;
                                        sessions.push(session);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(mut controls_map) = get_controls().lock() {
                for (name, data) in controls_map.drain() {
                    if !new_controls.contains_key(&name) {
                        let session_control = IAudioSessionControl::from_raw(data.session_control as *mut _);
                        let callback = IAudioSessionEvents::from_raw(data.callback as *mut _);
                        let _ = session_control.UnregisterAudioSessionNotification(&callback);

                        drop(callback);
                        drop(session_control);

                        let volume_control = ISimpleAudioVolume::from_raw(data.volume_control as *mut _);
                        drop(volume_control);
                    }
                }

                *controls_map = new_controls;
            }

            Ok(sessions)
        }
    }

    fn set_volume(&mut self, session_id: &str, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(controls) = get_controls().lock() {
            if let Some(data) = controls.get(session_id) {
                unsafe {
                    let control = ISimpleAudioVolume::from_raw(data.volume_control as *mut _);
                    control.SetMasterVolume(volume, &APP_CONTEXT_GUID as *const _)?;
                    std::mem::forget(control);
                }
            }
        }
        Ok(())
    }

    fn set_mute(&mut self, session_id: &str, muted: bool) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(controls) = get_controls().lock() {
            if let Some(data) = controls.get(session_id) {
                unsafe {
                    let control = ISimpleAudioVolume::from_raw(data.volume_control as *mut _);
                    control.SetMute(muted, &APP_CONTEXT_GUID as *const _)?;
                    std::mem::forget(control);
                }
            }
        }
        Ok(())
    }

    fn start_listening(&mut self, sender: mpsc::Sender<AudioUpdate>) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut state) = get_app_state().lock() {
            *state = Some(AppStateHandle { sender });
        }
        Ok(())
    }

    fn stop_listening(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut state) = get_app_state().lock() {
            *state = None;
        }
        Ok(())
    }
}