use std::collections::HashMap;
use std::io::Write;

use serialport::SerialPort;

use crate::comms::device_info::DeviceInfo;
use crate::comms::protocol::{
    CMD_DISPLAY_UPDATE_APP_NAME, CMD_DISPLAY_UPDATE_APP_VOLUME,
    DisplayUpdateAppCommand, DisplayUpdateVolumeCommand,
};

// ── Renames ───────────────────────────────────────────────────────────────────

pub fn save_device_renames(devices: &[DeviceInfo]) {
    let mut table = toml::map::Map::new();
    for dev in devices {
        if let Some(name) = &dev.rename {
            table.insert(DeviceInfo::uuid_str(&dev.uuid), toml::Value::String(name.clone()));
        }
    }
    save_section("renames", table);
}

pub fn load_device_renames() -> HashMap<String, String> {
    load_section("renames")
        .and_then(|v| if let toml::Value::Table(t) = v { Some(t) } else { None })
        .map(|t| t.into_iter()
            .filter_map(|(k, v)| if let toml::Value::String(s) = v { Some((k, s)) } else { None })
            .collect())
        .unwrap_or_default()
}

// ── Channel assignments ───────────────────────────────────────────────────────

pub fn save_device_assignments(devices: &[DeviceInfo]) {
    let mut table = toml::map::Map::new();
    for dev in devices {
        let arr: Vec<toml::Value> = dev.channel_assignments.iter()
            .map(|s| toml::Value::String(s.clone()))
            .collect();
        table.insert(DeviceInfo::uuid_str(&dev.uuid), toml::Value::Array(arr));
    }
    save_section("assignments", table);
}

pub fn load_device_assignments() -> HashMap<String, [String; 5]> {
    load_section("assignments")
        .and_then(|v| if let toml::Value::Table(t) = v { Some(t) } else { None })
        .map(|t| t.into_iter()
            .filter_map(|(k, v)| {
                if let toml::Value::Array(arr) = v {
                    let mut slots: [String; 5] = Default::default();
                    for (i, val) in arr.into_iter().take(5).enumerate() {
                        if let toml::Value::String(s) = val { slots[i] = s; }
                    }
                    Some((k, slots))
                } else {
                    None
                }
            })
            .collect())
        .unwrap_or_default()
}

// ── Serial send helpers ───────────────────────────────────────────────────────

pub fn send_app_name(port: &mut dyn SerialPort, channel: u8, name: &str) {
    let mut cmd = DisplayUpdateAppCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_NAME,
        channel,
        name: [0; 64],
    };
    let bytes = name.as_bytes();
    let len = bytes.len().min(63);
    cmd.name[..len].copy_from_slice(&bytes[..len]);

    let raw = unsafe {
        std::slice::from_raw_parts(
            &cmd as *const _ as *const u8,
            std::mem::size_of::<DisplayUpdateAppCommand>(),
        )
    };
    let _ = port.write_all(raw);
    let _ = port.flush();
}

pub fn send_volume(port: &mut dyn SerialPort, channel: u8, volume: u8) {
    let cmd = DisplayUpdateVolumeCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_VOLUME,
        channel,
        volume: volume.min(100),
    };
    let raw = unsafe {
        std::slice::from_raw_parts(
            &cmd as *const _ as *const u8,
            std::mem::size_of::<DisplayUpdateVolumeCommand>(),
        )
    };
    let _ = port.write_all(raw);
    let _ = port.flush();
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn config_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap_or_default();
    p.set_file_name("faderflow.toml");
    p
}

fn load_full() -> toml::map::Map<String, toml::Value> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .and_then(|v| if let toml::Value::Table(t) = v { Some(t) } else { None })
        .unwrap_or_default()
}

fn save_full(table: toml::map::Map<String, toml::Value>) {
    if let Ok(s) = toml::to_string(&toml::Value::Table(table)) {
        let _ = std::fs::write(config_path(), s);
    }
}

fn load_section(section: &str) -> Option<toml::Value> {
    load_full().remove(section)
}

fn save_section(section: &str, value: toml::map::Map<String, toml::Value>) {
    let mut full = load_full();
    full.insert(section.into(), toml::Value::Table(value));
    save_full(full);
}