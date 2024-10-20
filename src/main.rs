use std::collections::HashMap;

use wayland_client::{
    backend::ObjectId,
    event_created_child,
    protocol::{
        wl_output::Transform,
        wl_registry::{self, WlRegistry},
    },
    Connection, Dispatch, Proxy, WEnum,
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::{self, AdaptiveSyncState, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

fn main() {
    let connection = Connection::connect_to_env().expect("Failed to establish a connection");
    let display = connection.display();

    let mut event_queue = connection.new_event_queue();
    let qhandle = event_queue.handle();

    display.get_registry(&qhandle, ());

    let mut app_data = AppData::default();
    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

#[derive(Default)]
struct AppData {
    id_to_partial_head: HashMap<ObjectId, PartialHead>,
    id_to_head: HashMap<ObjectId, Head>,
    id_to_partial_mode: HashMap<ObjectId, PartialMode>,
    id_to_mode: HashMap<ObjectId, Mode>,
}

#[derive(Clone, Debug, Default)]
struct PartialHead {
    name: Option<String>,
    description: Option<String>,
    make: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    physical_size: Option<(u32, u32)>,
    enabled: Option<bool>,
    modes: Vec<ObjectId>,
    current_mode: Option<ObjectId>,
    position: Option<(u32, u32)>,
    transform: Option<WEnum<Transform>>,
    scale: Option<f64>,
    adaptive_sync: Option<WEnum<AdaptiveSyncState>>,
}

#[derive(Clone, Debug)]
struct Head {
    identity: HeadIdentity,
    modes: Vec<ObjectId>,
    configuration: Option<HeadConfiguration>,
}

#[derive(Clone, Debug)]
struct HeadIdentity {
    name: String,
    description: String,
    make: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    physical_size: Option<(u32, u32)>,
}

#[derive(Clone, Debug)]
struct HeadConfiguration {
    current_mode: ObjectId,
    position: (u32, u32),
    transform: WEnum<Transform>,
    scale: f64,
    adaptive_sync: Option<WEnum<AdaptiveSyncState>>,
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

#[derive(Clone, Copy, Debug, Default)]
struct PartialMode {
    size: Option<(u32, u32)>,
    refresh: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
struct Mode {
    size: (u32, u32),
    refresh: Option<u32>,
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

impl Dispatch<WlRegistry, ()> for AppData {
    fn event(
        _state: &mut Self,
        proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match &interface[..] {
                "zwlr_output_manager_v1" => {
                    proxy.bind::<zwlr_output_manager_v1::ZwlrOutputManagerV1, _, _>(
                        name,
                        version,
                        qhandle,
                        (),
                    );
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                state
                    .id_to_partial_head
                    .insert(head.id(), PartialHead::default());
                return;
            }
            zwlr_output_manager_v1::Event::Done { .. } => {}
            _ => return,
        }
        for (id, partial_head) in state.id_to_partial_head.drain() {
            state.id_to_head.insert(
                id,
                partial_head
                    .try_into()
                    .expect("Done is called, so the partial head should be well-defined"),
            );
        }
        println!("Heads: {:?}", state.id_to_head);
        for (id, partial_mode) in state.id_to_partial_mode.drain() {
            state.id_to_mode.insert(
                id,
                partial_mode
                    .try_into()
                    .expect("Done is called, so the partial mode should be well-defined"),
            );
        }
        println!("Modes: {:?}", state.id_to_mode);
        // TODO: Notify that the outputs are done.
    }

    event_created_child!(AppData, ZwlrOutputHeadV1, [
       zwlr_output_manager_v1::EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        enum HeadState<'a> {
            Partial(&'a mut PartialHead),
            Full(&'a mut Head),
        }
        let head_state = if let Some(partial_head) = state.id_to_partial_head.get_mut(&proxy.id()) {
            HeadState::Partial(partial_head)
        } else if let Some(head) = state.id_to_head.get_mut(&proxy.id()) {
            HeadState::Full(head)
        } else {
            panic!(
                "This proxy {} does not correspond to a previously existing head.",
                proxy.id()
            )
        };
        match event {
            zwlr_output_head_v1::Event::Finished => {
                state.id_to_partial_head.remove(&proxy.id());
                state.id_to_head.remove(&proxy.id());
                proxy.release();
            }
            zwlr_output_head_v1::Event::Name { name } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event Name for head {}, which is already done:w
                    ",
                        proxy.id()
                    );
                };
                partial_head.name = Some(name);
            }
            zwlr_output_head_v1::Event::Description { description } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event Description for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.description = Some(description);
            }
            zwlr_output_head_v1::Event::Make { make } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event Make for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.make = Some(make);
            }
            zwlr_output_head_v1::Event::Model { model } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event Model for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.model = Some(model);
            }
            zwlr_output_head_v1::Event::SerialNumber { serial_number } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event SerialNumber for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.serial_number = Some(serial_number);
            }
            zwlr_output_head_v1::Event::PhysicalSize { width, height } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received identity event PhysicalSize for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.physical_size = Some((width as u32, height as u32));
            }
            zwlr_output_head_v1::Event::Mode { mode } => {
                let HeadState::Partial(partial_head) = head_state else {
                    panic!(
                        "Received event Mode for head {}, which is already done",
                        proxy.id()
                    );
                };
                partial_head.modes.push(mode.id());
                state
                    .id_to_partial_mode
                    .insert(mode.id(), PartialMode::default());
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                let enabled = enabled > 0;
                match head_state {
                    HeadState::Partial(partial_head) => {
                        partial_head.enabled = Some(enabled);
                    }
                    HeadState::Full(head) => {
                        head.configuration = None;
                    }
                }
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => match head_state {
                HeadState::Partial(partial_head) => {
                    partial_head.current_mode = Some(mode.id());
                }
                HeadState::Full(head) => {
                    let configuration = head
                        .configuration
                        .as_mut()
                        .expect("Received a CurrentMode event while head is disabled");
                    configuration.current_mode = mode.id();
                }
            },
            zwlr_output_head_v1::Event::Position { x, y } => match head_state {
                HeadState::Partial(partial_head) => {
                    partial_head.position = Some((x as u32, y as u32));
                }
                HeadState::Full(head) => {
                    let configuration = head
                        .configuration
                        .as_mut()
                        .expect("Received a Position event while head is disabled");
                    configuration.position = (x as u32, y as u32);
                }
            },
            zwlr_output_head_v1::Event::Transform { transform } => match head_state {
                HeadState::Partial(partial_head) => {
                    partial_head.transform = Some(transform);
                }
                HeadState::Full(head) => {
                    let configuration = head
                        .configuration
                        .as_mut()
                        .expect("Received a Transform event while head is disabled");
                    configuration.transform = transform;
                }
            },
            zwlr_output_head_v1::Event::Scale { scale } => match head_state {
                HeadState::Partial(partial_head) => {
                    partial_head.scale = Some(scale);
                }
                HeadState::Full(head) => {
                    let configuration = head
                        .configuration
                        .as_mut()
                        .expect("Received a Scale event while head is disabled");
                    configuration.scale = scale;
                }
            },
            zwlr_output_head_v1::Event::AdaptiveSync { state } => match head_state {
                HeadState::Partial(partial_head) => {
                    partial_head.adaptive_sync = Some(state);
                }
                HeadState::Full(head) => {
                    let configuration = head
                        .configuration
                        .as_mut()
                        .expect("Received a AdaptiveSync event while head is disabled");
                    configuration.adaptive_sync = Some(state);
                }
            },
            _ => {}
        }
    }

    event_created_child!(AppData, ZwlrOutputModeV1, [
        zwlr_output_head_v1::EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputModeV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let id = proxy.id();
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                let partial_mode = state
                    .id_to_partial_mode
                    .get_mut(&id)
                    .expect("The mode was previously reported and not finished.");
                partial_mode.size = Some((width as u32, height as u32));
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                let partial_mode = state
                    .id_to_partial_mode
                    .get_mut(&id)
                    .expect("The mode was previously reported and not finished.");
                partial_mode.refresh = Some(refresh as u32);
            }
            zwlr_output_mode_v1::Event::Finished => {
                state.id_to_partial_mode.remove(&id);
                state.id_to_mode.remove(&id);
                proxy.release();
            }
            _ => {}
        }
    }
}
