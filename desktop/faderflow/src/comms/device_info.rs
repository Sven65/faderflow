use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::comms::scanner::SharedPort;

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceStatus {
    Connected,
    Lost,
}

pub struct DeviceInfo {
    pub port_name: String,
    pub port: SharedPort,
    pub uuid: [u8; 16],
    pub version: (u8, u8),
    pub rename: Option<String>,
    pub status: DeviceStatus,
    pub watchdog_cancel: Arc<AtomicBool>,
}

impl DeviceInfo {
    pub fn display_name(&self) -> &str {
        self.rename.as_deref().unwrap_or(&self.port_name)
    }

    /// Formats a UUID byte array as "AABBCCDD-EEFFGGHH-..."
    pub fn uuid_str(uuid: &[u8; 16]) -> String {
        uuid.chunks(4)
            .map(|c| c.iter().map(|b| format!("{b:02X}")).collect::<String>())
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn uuid_string(&self) -> String {
        Self::uuid_str(&self.uuid)
    }

    pub fn version_string(&self) -> String {
        format!("v{}.{}", self.version.0, self.version.1)
    }

    pub fn cancel_watchdog(&self) {
        self.watchdog_cancel.store(true, Ordering::Relaxed);
    }
}