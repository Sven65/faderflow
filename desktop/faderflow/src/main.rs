mod comms;
mod ui;
mod audio;
mod utils;

use serialport::{self};
use std::io::{Read, Write, BufRead};
use ui::resources::load_icon;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use iced::application::TitleFn;
use iced::Settings;
use comms::protocol;
use crate::comms::protocol::{
    FaderMessage, HandshakeResponse,
    CMD_FADER_UPDATE, CMD_HANDSHAKE_REQUEST, CMD_HANDSHAKE_ACK, CMD_HANDSHAKE_RESPONSE,
    DisplayUpdateAppCommand, DisplayUpdateVolumeCommand,
    CMD_DISPLAY_UPDATE_APP_NAME, CMD_DISPLAY_UPDATE_APP_VOLUME
};

pub const INIT_WAIT_TIME_MS: u64 = 100;

use ui::app::VolumeApp;

fn main() -> iced::Result {
    iced::application(VolumeApp::new, VolumeApp::update, VolumeApp::view)
        .subscription(VolumeApp::subscription)
        .title("FaderFlow")
        .window(iced::window::Settings {
            icon: Some(load_icon()),
            ..Default::default()
        })
        .run()
}

/*
fn send_app_name(port: &mut dyn SerialPort, channel: u8, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = DisplayUpdateAppCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_NAME,
        channel,
        name: [0; 64],
    };

    let bytes = name.as_bytes();
    let len = bytes.len().min(63);
    cmd.name[..len].copy_from_slice(&bytes[..len]);

    let bytes = unsafe {
        std::slice::from_raw_parts(&cmd as *const _ as *const u8, std::mem::size_of::<DisplayUpdateAppCommand>())
    };

    port.write_all(bytes)?;
    port.flush()?;

    println!("Sent app name '{}' to channel {}", name, channel);
    Ok(())
}

fn send_volume(port: &mut dyn SerialPort, channel: u8, volume: u8) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = DisplayUpdateVolumeCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_VOLUME,
        channel,
        volume: volume.min(100),
    };

    let bytes = unsafe {
        std::slice::from_raw_parts(&cmd as *const _ as *const u8, std::mem::size_of::<DisplayUpdateVolumeCommand>())
    };
    port.write_all(bytes)?;
    port.flush()?;

    println!("Sent volume {} to channel {}", volume, channel);
    Ok(())
}*/