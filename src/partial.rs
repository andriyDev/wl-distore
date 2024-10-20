use std::collections::HashMap;

use wayland_client::{backend::ObjectId, protocol::wl_output::Transform, WEnum};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;

#[derive(Clone, Debug, Default)]
pub struct PartialHead {
    pub name: Option<String>,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub physical_size: Option<(u32, u32)>,
    pub enabled: Option<bool>,
    pub modes: Vec<ObjectId>,
    pub current_mode: Option<ObjectId>,
    pub position: Option<(u32, u32)>,
    pub transform: Option<WEnum<Transform>>,
    pub scale: Option<f64>,
    pub adaptive_sync: Option<WEnum<AdaptiveSyncState>>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PartialMode {
    pub size: Option<(u32, u32)>,
    pub refresh: Option<u32>,
}

#[derive(Default)]
pub struct PartialObjects {
    pub id_to_head: HashMap<ObjectId, PartialHead>,
    pub id_to_mode: HashMap<ObjectId, PartialMode>,
}
