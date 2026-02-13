// protocol.rs - Shared protocol definitions matching Arduino protocol.h

pub const CMD_HANDSHAKE_REQUEST: u8 = 0x01;
pub const CMD_ECHO_UUID: u8 = 0x02;

pub const  CMD_DISPLAY_UPDATE_APP_NAME: u8 = 0x03;
pub const  CMD_DISPLAY_UPDATE_APP_VOLUME: u8 = 0x04;
pub const  CMD_DISPLAY_UPDATE_ICON: u8 = 0x05;

pub const CMD_FADER_UPDATE: u8 = 0x10;

pub const MAGIC_STRING: &[u8] = b"FADERFLOW";
pub const UUID_SIZE: usize = 16;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HandshakeResponse {
    pub magic: [u8; 10],
    pub device_type: u8,
    pub uuid: [u8; UUID_SIZE],
    pub version_major: u8,
    pub version_minor: u8,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct FaderMessage {
    pub cmd: u8,
    pub channel: u8,
    pub position: u8,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct DisplayUpdateAppCommand {
   pub cmd: u8,        // CMD_SET_APP
   pub channel: u8,
   pub name: [u8; 64],
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct DisplayUpdateVolumeCommand {
    pub cmd: u8,        // CMD_SET_VOLUME
    pub channel: u8,
    pub volume: u8,     // 0-100
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct DisplayUpdateIconCommand {
    pub cmd: u8,        // CMD_SET_ICON
    pub channel: u8,
    // Followed by: uint8_t iconData[8192]
}

impl HandshakeResponse {
    pub fn is_valid(&self) -> bool {
        &self.magic[..9] == MAGIC_STRING
    }
}

impl FaderMessage {
    pub fn position_percent(&self) -> u8 {
        (self.position as f32 / 255.0 * 100.0) as u8
    }
}