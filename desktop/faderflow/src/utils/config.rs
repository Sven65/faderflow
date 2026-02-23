use std::collections::HashMap;

use crate::comms::device_info::DeviceInfo;

pub fn save_device_renames(devices: &[DeviceInfo]) {
    let mut table = toml::map::Map::new();
    for dev in devices {
        if let Some(name) = &dev.rename {
            table.insert(DeviceInfo::uuid_str(&dev.uuid), toml::Value::String(name.clone()));
        }
    }
    let s = toml::to_string(&toml::Value::Table(table)).unwrap_or_default();
    let _ = std::fs::write(config_path(), s);
}

pub fn load_device_renames() -> HashMap<String, String> {
    let Ok(s) = std::fs::read_to_string(config_path()) else { return HashMap::new(); };
    let Ok(toml::Value::Table(t)) = toml::from_str(&s) else { return HashMap::new(); };
    t.into_iter()
        .filter_map(|(k, v)| if let toml::Value::String(s) = v { Some((k, s)) } else { None })
        .collect()
}

fn config_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap_or_default();
    p.set_file_name("faderflow_devices.toml");
    p
}
