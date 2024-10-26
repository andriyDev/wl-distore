use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wayland_client::backend::ObjectId;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use crate::{
    partial::{
        ConfigurationProperty, ImmutableProperty, PartialHead, PartialHeadState, PartialMode,
        PartialModeState,
    },
    serde::Transform,
};

pub struct HeadState {
    pub proxy: ZwlrOutputHeadV1,
    pub head: Head,
}

#[derive(Clone, Debug)]
pub struct Head {
    pub identity: HeadIdentity,
    pub mode_to_id: HashMap<Mode, ObjectId>,
    pub configuration: Option<HeadConfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeadIdentity {
    pub name: String,
    pub description: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HeadConfiguration {
    pub current_mode: Option<ObjectId>,
    pub position: (u32, u32),
    pub transform: Transform,
    pub scale: f64,
    pub adaptive_sync: Option<bool>,
}

impl Default for HeadConfiguration {
    fn default() -> Self {
        Self {
            current_mode: None,
            position: (0, 0),
            transform: Transform::Normal,
            scale: 1.0,
            adaptive_sync: None,
        }
    }
}

impl Head {
    // TODO: Make an actual error type.
    fn create_from_partial(
        mut value: PartialHead,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) -> Result<Self, ()> {
        let Some(name) = std::mem::take(&mut value.name) else {
            return Err(());
        };
        let Some(description) = std::mem::take(&mut value.description) else {
            return Err(());
        };
        if value.enabled.is_none() {
            // Make sure the first instance gets the Enabled event.
            return Err(());
        }

        let mut head = Self {
            identity: HeadIdentity {
                name,
                description,
                make: std::mem::take(&mut value.make),
                model: std::mem::take(&mut value.model),
                serial_number: std::mem::take(&mut value.serial_number),
            },
            mode_to_id: Default::default(),
            configuration: None,
        };

        head.apply_partial(value, id_to_mode).map_err(|_| ())?;
        Ok(head)
    }

    /// Sets the values in `partial` on `self`. Returns an error if any immutable property is set,
    /// or a disabled head has any configuration properties set on `partial`.
    pub fn apply_partial(
        &mut self,
        partial: PartialHead,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) -> Result<(), ApplyPartialHeadError> {
        if let Some(immutable_property) = partial.get_assigned_immutable_property() {
            return Err(ApplyPartialHeadError::ImmutablePropertySet(
                immutable_property,
            ));
        }

        self.mode_to_id.extend(partial.modes.iter().map(|id| {
            (
                id_to_mode
                    .get(id)
                    .map(|mode_state| mode_state.mode.clone())
                    .expect("Head contains unknown mode"),
                id.clone(),
            )
        }));

        if let Some(enabled) = partial.enabled {
            if !enabled {
                self.configuration = None;

                if let Some(configuration_property) = partial.get_assigned_configuration_property()
                {
                    return Err(
                        ApplyPartialHeadError::ConfigurationPropertyOnDisabledHeadSet(
                            configuration_property,
                        ),
                    );
                }
                return Ok(());
            } else {
                self.configuration = Some(Default::default());
            }
        }

        let Some(configuration) = self.configuration.as_mut() else {
            // Either a head was already disabled, in which we shouldn't have gotten any
            // configuration events, or the head just got disabled, so we already returned earlier.
            if let Some(configuration_property) = partial.get_assigned_configuration_property() {
                return Err(
                    ApplyPartialHeadError::ConfigurationPropertyOnDisabledHeadSet(
                        configuration_property,
                    ),
                );
            }
            return Ok(());
        };

        configuration.current_mode = partial.current_mode;
        if let Some(position) = partial.position {
            configuration.position = position;
        }
        if let Some(transform) = partial.transform {
            configuration.transform = transform;
        }
        if let Some(scale) = partial.scale {
            configuration.scale = scale;
        }
        configuration.adaptive_sync = partial.adaptive_sync;

        Ok(())
    }
}

impl HeadState {
    // TODO: Make an actual error type.
    pub fn create_from_partial(
        value: PartialHeadState,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) -> Result<Self, ()> {
        Ok(Self {
            proxy: value.proxy,
            head: Head::create_from_partial(value.head, id_to_mode)?,
        })
    }
}

#[derive(Debug, Error)]
pub enum ApplyPartialHeadError {
    #[error("The immutable property {0:?} is set, trying to mutate an existing head.")]
    ImmutablePropertySet(ImmutableProperty),
    #[error("The configuration property {0:?} is set on a disabled head.")]
    ConfigurationPropertyOnDisabledHeadSet(ConfigurationProperty),
}

pub struct ModeState {
    pub proxy: ZwlrOutputModeV1,
    pub mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mode {
    pub size: (u32, u32),
    pub refresh: Option<u32>,
}

impl TryFrom<PartialMode> for Mode {
    // TODO: Make an actual error type.
    type Error = ();

    fn try_from(value: PartialMode) -> Result<Self, Self::Error> {
        let Some(size) = value.size else {
            return Err(());
        };
        Ok(Self {
            size,
            refresh: value.refresh,
        })
    }
}

impl TryFrom<PartialModeState> for ModeState {
    // TODO: Make an actual error type.
    type Error = ();

    fn try_from(value: PartialModeState) -> Result<Self, Self::Error> {
        Ok(Self {
            proxy: value.proxy,
            mode: value.mode.try_into()?,
        })
    }
}
