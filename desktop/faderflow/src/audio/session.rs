use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct AudioSession {
    pub id: String,
    pub display_name: String,
    pub volume: f32,
    pub is_muted: bool,
    pub process_id: u32,
    pub icon_handle: Option<Arc<iced::widget::image::Handle>>,
    pub last_local_change: Option<Instant>,
    pub last_external_change: Option<Instant>,
}

impl AudioSession {
    pub fn new(
        id: String,
        display_name: String,
        volume: f32,
        is_muted: bool,
        process_id: u32,
    ) -> Self {
        Self {
            id,
            display_name,
            volume,
            is_muted,
            process_id,
            icon_handle: None,
            last_local_change: None,
            last_external_change: None,
        }
    }
}
