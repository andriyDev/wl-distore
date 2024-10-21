use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use wayland_client::{backend::ObjectId, protocol::wl_output::Transform as wayland_Transform};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_head_v1::AdaptiveSyncState,
};

use crate::complete::{HeadConfiguration, HeadIdentity, Mode, ModeState};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Transform {
    Normal,
    _90,
    _180,
    _270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

impl TryFrom<wayland_Transform> for Transform {
    // TODO: Make an actual error type.
    type Error = ();

    fn try_from(value: wayland_Transform) -> Result<Self, Self::Error> {
        Ok(match value {
            wayland_Transform::Normal => Self::Normal,
            wayland_Transform::_90 => Self::_90,
            wayland_Transform::_180 => Self::_180,
            wayland_Transform::_270 => Self::_270,
            wayland_Transform::Flipped => Self::Flipped,
            wayland_Transform::Flipped90 => Self::Flipped90,
            wayland_Transform::Flipped180 => Self::Flipped180,
            wayland_Transform::Flipped270 => Self::Flipped270,
            _ => return Err(()),
        })
    }
}

impl Into<wayland_Transform> for Transform {
    fn into(self) -> wayland_Transform {
        match self {
            Self::Normal => wayland_Transform::Normal,
            Self::_90 => wayland_Transform::_90,
            Self::_180 => wayland_Transform::_180,
            Self::_270 => wayland_Transform::_270,
            Self::Flipped => wayland_Transform::Flipped,
            Self::Flipped90 => wayland_Transform::Flipped90,
            Self::Flipped180 => wayland_Transform::Flipped180,
            Self::Flipped270 => wayland_Transform::Flipped270,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedConfiguration {
    mode: Mode,
    position: (u32, u32),
    transform: Transform,
    scale: f64,
    adaptive_sync: Option<bool>,
}

impl SavedConfiguration {
    pub fn from_config(
        configuration: &HeadConfiguration,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) -> Self {
        SavedConfiguration {
            mode: id_to_mode
                .get(&configuration.current_mode)
                .expect("The current mode doesn't exist.")
                .mode
                .clone(),
            position: configuration.position,
            transform: configuration.transform,
            scale: configuration.scale,
            adaptive_sync: configuration.adaptive_sync,
        }
    }

    // TODO: Make a real error type.
    pub fn apply(
        &self,
        new_configuration_head: &mut ZwlrOutputConfigurationHeadV1,
        mode_to_id: &HashMap<Mode, ObjectId>,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) {
        if let Some(id) = mode_to_id.get(&self.mode).cloned() {
            let proxy = &id_to_mode
                .get(&id)
                .expect("Missing mode for existing id")
                .proxy;
            new_configuration_head.set_mode(proxy);
        } else {
            new_configuration_head.set_custom_mode(
                self.mode.size.0 as i32,
                self.mode.size.1 as i32,
                self.mode.refresh.unwrap_or(0) as i32,
            );
        }
        new_configuration_head.set_position(self.position.0 as i32, self.position.1 as i32);
        new_configuration_head.set_scale(self.scale);
        new_configuration_head.set_transform(self.transform.into());
        if let Some(adaptive_sync) = self.adaptive_sync {
            new_configuration_head.set_adaptive_sync(if adaptive_sync {
                AdaptiveSyncState::Enabled
            } else {
                AdaptiveSyncState::Disabled
            });
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct LayoutData {
    pub layouts: Vec<HashMap<HeadIdentity, Option<SavedConfiguration>>>,
}
