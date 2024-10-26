use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, ErrorKind},
    path::Path,
};

use serde::{Deserialize, Serialize};

use thiserror::Error;
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
    type Error = TransformConversionError;

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
            value => return Err(TransformConversionError::UnknownVariant(value)),
        })
    }
}

#[derive(Debug, Error)]
pub enum TransformConversionError {
    #[error("An unknown Transform variant was received: {0:?}")]
    UnknownVariant(wayland_Transform),
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
    mode: Option<Mode>,
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
            mode: configuration.current_mode.as_ref().map(|mode| {
                id_to_mode
                    .get(&mode)
                    .expect("The current mode doesn't exist.")
                    .mode
                    .clone()
            }),
            position: configuration.position,
            transform: configuration.transform,
            scale: configuration.scale,
            adaptive_sync: configuration.adaptive_sync,
        }
    }

    pub fn apply(
        &self,
        new_configuration_head: &mut ZwlrOutputConfigurationHeadV1,
        mode_to_id: &HashMap<Mode, ObjectId>,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) {
        if let Some(mode) = self.mode {
            if let Some(id) = mode_to_id.get(&mode).cloned() {
                let proxy = &id_to_mode
                    .get(&id)
                    .expect("Missing mode for existing id")
                    .proxy;
                new_configuration_head.set_mode(proxy);
            } else {
                new_configuration_head.set_custom_mode(
                    mode.size.0 as i32,
                    mode.size.1 as i32,
                    mode.refresh.unwrap_or(0) as i32,
                );
            }
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

pub struct LayoutData {
    pub layouts: Vec<HashMap<HeadIdentity, Option<SavedConfiguration>>>,
}

impl LayoutData {
    /// Loads an instance from `path`. Returns an empty instance if the file is not found (since
    /// that indicates this is the first run).
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(err) => {
                return if err.kind() == ErrorKind::NotFound {
                    Ok(Self {
                        layouts: Default::default(),
                    })
                } else {
                    Err(err)
                }
            }
        };
        let saved_layout_data: SavedLayoutData = serde_json::from_reader(BufReader::new(file))?;
        Ok((&saved_layout_data).into())
    }

    /// Saves self to the file at `path`.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        let saved_layout_data: SavedLayoutData = self.into();
        serde_json::to_writer(BufWriter::new(file), &saved_layout_data)?;
        Ok(())
    }
}

#[derive(Default, Serialize, Deserialize)]
struct SavedLayoutData {
    layouts: Vec<Vec<(HeadIdentity, Option<SavedConfiguration>)>>,
}

impl From<&SavedLayoutData> for LayoutData {
    fn from(value: &SavedLayoutData) -> Self {
        Self {
            layouts: value
                .layouts
                .iter()
                .map(|entries| entries.iter().cloned().collect())
                .collect(),
        }
    }
}

impl From<&LayoutData> for SavedLayoutData {
    fn from(value: &LayoutData) -> Self {
        Self {
            layouts: value
                .layouts
                .iter()
                .map(|entries| {
                    entries
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .collect(),
        }
    }
}
