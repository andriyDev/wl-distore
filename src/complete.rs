use wayland_client::{backend::ObjectId, protocol::wl_output::Transform, WEnum};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;

use crate::partial::{PartialHead, PartialMode};

#[derive(Clone, Debug)]
pub struct Head {
    pub identity: HeadIdentity,
    pub modes: Vec<ObjectId>,
    pub configuration: Option<HeadConfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    pub current_mode: ObjectId,
    pub position: (u32, u32),
    pub transform: WEnum<Transform>,
    pub scale: f64,
    pub adaptive_sync: Option<WEnum<AdaptiveSyncState>>,
}

impl TryFrom<PartialHead> for Head {
    // TODO: Make an actual error type.
    type Error = ();

    fn try_from(value: PartialHead) -> Result<Self, Self::Error> {
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
            let Some(current_mode) = value.current_mode else {
                return Err(());
            };
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
                current_mode,
                position,
                transform,
                scale,
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
            modes: value.modes,
            configuration,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
