// protocol.rs - Shared protocol definitions matching Arduino protocol.h

pub const CMD_HANDSHAKE_REQUEST: u8 = 0x01;
pub const CMD_ECHO_UUID: u8 = 0x02;
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