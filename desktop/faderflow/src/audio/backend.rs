use super::session::AudioSession;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AudioUpdate {
    VolumeChanged(String, f32),
    MuteChanged(String, bool),
    SessionAdded(AudioSession),
    SessionRemoved(String),
    DefaultDeviceChanged(String), // friendly name of new default output
}

pub trait AudioBackend: Send + Sync {
    /// Initialize the audio backend
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Get all current audio sessions
    fn get_sessions(&self) -> Result<Vec<AudioSession>, Box<dyn std::error::Error>>;

    /// Set volume for a session (0.0 to 1.0)
    fn set_volume(&mut self, session_id: &str, volume: f32) -> Result<(), Box<dyn std::error::Error>>;

    /// Set mute state for a session
    fn set_mute(&mut self, session_id: &str, muted: bool) -> Result<(), Box<dyn std::error::Error>>;

    /// Start listening for audio events
    fn start_listening(&mut self, sender: mpsc::Sender<AudioUpdate>) -> Result<(), Box<dyn std::error::Error>>;

    /// Stop listening for audio events
    fn stop_listening(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Get all active output (render) devices by friendly name
    fn get_output_devices(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    /// Get the friendly name of the current default output device
    fn get_default_output_device(&self) -> Option<String>;
}