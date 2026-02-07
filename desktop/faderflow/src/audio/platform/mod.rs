#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use super::backend::AudioBackend;

pub fn create_backend() -> Box<dyn AudioBackend> {
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsAudioBackend::new());

    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxAudioBackend::new());

    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSAudioBackend::new());

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    compile_error!("Unsupported platform");
}