use serde::{Deserialize, Serialize};

use wayland_client::protocol::wl_output::Transform as wayland_Transform;

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
