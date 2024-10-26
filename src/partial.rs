use std::collections::HashMap;

use wayland_client::backend::ObjectId;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use crate::serde::Transform;

#[derive(Clone, Debug, Default)]
pub struct PartialHead {
    pub name: Option<String>,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub enabled: Option<bool>,
    pub modes: Vec<ObjectId>,
    pub current_mode: Option<ObjectId>,
    pub position: Option<(u32, u32)>,
    pub transform: Option<Transform>,
    pub scale: Option<f64>,
    pub adaptive_sync: Option<bool>,
}

impl PartialHead {
    pub fn get_assigned_immutable_property(&self) -> Option<ImmutableProperty> {
        if self.name.is_some() {
            Some(ImmutableProperty::Name)
        } else if self.description.is_some() {
            Some(ImmutableProperty::Description)
        } else if self.make.is_some() {
            Some(ImmutableProperty::Make)
        } else if self.model.is_some() {
            Some(ImmutableProperty::Model)
        } else if self.serial_number.is_some() {
            Some(ImmutableProperty::SerialNumber)
        } else {
            None
        }
    }

    pub fn get_assigned_configuration_property(&self) -> Option<ConfigurationProperty> {
        if self.current_mode.is_some() {
            Some(ConfigurationProperty::CurrentMode)
        } else if self.position.is_some() {
            Some(ConfigurationProperty::Position)
        } else if self.transform.is_some() {
            Some(ConfigurationProperty::Transform)
        } else if self.scale.is_some() {
            Some(ConfigurationProperty::Scale)
        } else if self.adaptive_sync.is_some() {
            Some(ConfigurationProperty::AdaptiveSync)
        } else {
            None
        }
    }
}

/// A property that is immutable after a head has been created.
#[derive(Debug, Clone, Copy)]
pub enum ImmutableProperty {
    Name,
    Description,
    Make,
    Model,
    SerialNumber,
}

/// A property about the configuration of an enabled head. Note we intentionally exclude Enabled.
#[derive(Debug, Clone, Copy)]
pub enum ConfigurationProperty {
    CurrentMode,
    Position,
    Transform,
    Scale,
    AdaptiveSync,
}

pub struct PartialHeadState {
    pub proxy: ZwlrOutputHeadV1,
    pub head: PartialHead,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PartialMode {
    pub size: Option<(u32, u32)>,
    pub refresh: Option<u32>,
}

pub struct PartialModeState {
    pub proxy: ZwlrOutputModeV1,
    pub mode: PartialMode,
}

#[derive(Default)]
pub struct PartialObjects {
    pub id_to_head: HashMap<ObjectId, PartialHeadState>,
    pub id_to_mode: HashMap<ObjectId, PartialModeState>,
}
