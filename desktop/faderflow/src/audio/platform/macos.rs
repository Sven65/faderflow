#[cfg(target_os = "macos")]
use crate::audio::{AudioBackend, AudioSession, AudioUpdate};
use std::sync::mpsc;

pub struct MacOSAudioBackend;

impl MacOSAudioBackend {
    pub fn new() -> Self {
        Self
    }
}

impl AudioBackend for MacOSAudioBackend {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Initialize CoreAudio
        Err("macOS support not yet implemented".into())
    }

    fn get_sessions(&self) -> Result<Vec<AudioSession>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    fn set_volume(&mut self, _session_id: &str, _volume: f32) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn set_mute(&mut self, _session_id: &str, _muted: bool) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn start_listening(&mut self, _sender: mpsc::Sender<AudioUpdate>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn stop_listening(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn get_output_devices(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }

    fn get_default_output_device(&self) -> Option<String> {
        None
    }
}