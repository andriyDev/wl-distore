use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wayland_client::backend::ObjectId;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use crate::{
    partial::{PartialHead, PartialHeadState, PartialMode, PartialModeState},
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
    pub physical_size: Option<(u32, u32)>,
}

#[derive(Clone, Debug)]
pub struct HeadConfiguration {
    pub current_mode: Option<ObjectId>,
    pub position: (u32, u32),
    pub transform: Transform,
    pub scale: f64,
    pub adaptive_sync: Option<bool>,
}

impl Head {
    // TODO: Make an actual error type.
    fn create_from_partial(
        value: PartialHead,
        id_to_mode: &HashMap<ObjectId, ModeState>,
    ) -> Result<Self, ()> {
        let Some(name) = value.name else {
            return Err(());
        };
        let Some(description) = value.description else {
            return Err(());
        };
        let Some(enabled) = value.enabled else {
            return Err(());
        };

        let mut configuration = None;
        if enabled {
            let Some(position) = value.position else {
                return Err(());
            };
            let Some(transform) = value.transform else {
                return Err(());
            };
            let Some(scale) = value.scale else {
                return Err(());
            };
            configuration = Some(HeadConfiguration {
                position,
                transform,
                scale,
                current_mode: value.current_mode,
                adaptive_sync: value.adaptive_sync,
            });
        }

        Ok(Head {
            identity: HeadIdentity {
                name,
                description,
                make: value.make,
                model: value.model,
                serial_number: value.serial_number,
                physical_size: value.physical_size,
            },
            mode_to_id: value
                .modes
                .iter()
                .map(|id| {
                    (
                        id_to_mode
                            .get(id)
                            .map(|mode_state| mode_state.mode.clone())
                            .expect("Head contains unknown mode"),
                        id.clone(),
                    )
                })
                .collect(),
            configuration,
        })
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
