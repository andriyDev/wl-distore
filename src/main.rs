use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    process::Command,
    sync::Arc,
};

use complete::{HeadIdentity, HeadState, ModeState};
use config::{Args, CollectArgsError};
use partial::{PartialHead, PartialHeadState, PartialModeState, PartialObjects};
use serde::{LayoutData, SavedConfiguration};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use wayland_client::{
    backend::ObjectId,
    event_created_child,
    protocol::wl_registry::{self, WlRegistry},
    Connection, Dispatch, Proxy,
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self, AdaptiveSyncState, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

mod complete;
mod config;
mod partial;
mod serde;

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = match Args::collect() {
        Ok(args) => args,
        Err(CollectArgsError::LayoutsPathIsDirectory(path)) => {
            eprintln!("Layouts file cannot be a directory: \"{}\"", path);
            std::process::exit(1);
        }
        err => err.expect("Failed to collect arguments"),
    };

    main_with_args(args);
}

fn main_with_args(args: Args) {
    let connection = Connection::connect_to_env().expect("Failed to establish a connection");
    let display = connection.display();

    let mut event_queue = connection.new_event_queue();
    let qhandle = event_queue.handle();

    display.get_registry(&qhandle, ());

    let mut app_data = AppData::new(args).expect("Failed to load layouts");
    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

struct AppData {
    args: Args,

    partial_objects: PartialObjects,
    id_to_head: HashMap<ObjectId, HeadState>,
    head_identity_to_id: HashMap<HeadIdentity, ObjectId>,
    id_to_mode: HashMap<ObjectId, ModeState>,
    done_action: DoneAction,
    layout_data: LayoutData,
}

#[derive(Default, Clone, Copy)]
enum DoneAction {
    /// Update the layout for the current head setup.
    #[default]
    Update,
    /// Apply the layout for the current head setup.
    Apply,
    /// The next Done events corresponds to the result of an Apply action, so ignore it.
    ApplyResult,
}

impl AppData {
    fn new(args: Args) -> Result<Self, std::io::Error> {
        Ok(Self {
            partial_objects: Default::default(),
            id_to_head: Default::default(),
            head_identity_to_id: Default::default(),
            id_to_mode: Default::default(),
            done_action: Default::default(),
            layout_data: LayoutData::load(&args.layouts)?,
            // Move after we load the layout data.
            args,
        })
    }

    fn save_layouts(&self) {
        self.layout_data
            .save(&self.args.layouts)
            .expect("Failed to save layouts");
    }

    /// Applies the layout at `index`. `serial` is the serial value provided from the most recent
    /// `Done` event.
    fn apply_layout(
        &mut self,
        index: usize,
        layout_head_to_query_head: HashMap<HeadIdentity, HeadIdentity>,
        output_manager: &ZwlrOutputManagerV1,
        qhandle: &wayland_client::QueueHandle<Self>,
        serial: u32,
    ) {
        self.done_action = DoneAction::ApplyResult;
        let identity_to_configuration = &self.layout_data.layouts[index];
        let new_configuration = output_manager.create_configuration(serial, qhandle, ());
        for (identity, configuration) in identity_to_configuration.iter() {
            // See if the layout head needs to be remapped to a query head, falling back to the
            // identity on failure.
            let identity = layout_head_to_query_head.get(identity).unwrap_or(identity);

            let id = self
                .head_identity_to_id
                .get(identity)
                .expect("Could not find head for matched layout");

            let head_state = &self
                .id_to_head
                .get(&id)
                .expect("Could not find proxy for id");

            match configuration.as_ref() {
                None => {
                    new_configuration.disable_head(&head_state.proxy);
                }
                Some(configuration) => {
                    let mut new_configuration_head =
                        new_configuration.enable_head(&head_state.proxy, qhandle, ());
                    configuration.apply(
                        &mut new_configuration_head,
                        &head_state.head.mode_to_id,
                        &self.id_to_mode,
                    );
                }
            }
        }
        new_configuration.apply();
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
        proxy: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        debug!("Received Manager event: {event:?}");
        let serial = match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                // A new head was added, so try to apply a layout on the next `Done` event.
                state.done_action = DoneAction::Apply;
                state.partial_objects.id_to_head.insert(
                    head.id(),
                    PartialHeadState {
                        proxy: head,
                        head: Default::default(),
                    },
                );
                return;
            }
            zwlr_output_manager_v1::Event::Done { serial } => serial,
            _ => return,
        };
        for (id, partial_mode) in state.partial_objects.id_to_mode.drain() {
            let mode_proxy = partial_mode.proxy.clone();
            let mode = match partial_mode.try_into() {
                Ok(mode) => mode,
                Err(err) => {
                    // Sway can create "phantom" modes, so just log any errors and release the
                    // offending modes. https://github.com/swaywm/sway/issues/8420
                    error!("Failed to convert partial mode into full mode: {err}");
                    mode_proxy.release();
                    continue;
                }
            };
            state.id_to_mode.insert(id, mode);
        }
        for (id, partial_head) in state.partial_objects.id_to_head.drain() {
            match state.id_to_head.entry(id.clone()) {
                Entry::Vacant(entry) => {
                    let head: HeadState =
                        HeadState::create_from_partial(partial_head, &state.id_to_mode)
                            .expect("Done is called, so the partial head should be well-defined");
                    assert!(
                        state
                            .head_identity_to_id
                            .insert(head.head.identity.clone(), id)
                            .is_none(),
                        "Head identities should be unique."
                    );
                    entry.insert(head);
                }
                Entry::Occupied(mut entry) => {
                    entry
                        .get_mut()
                        .head
                        .apply_partial(partial_head.head, &state.id_to_mode)
                        .expect("Failed to apply partial to existing head.");
                }
            }
        }

        let current_layout = state
            .id_to_head
            .values()
            .map(|head| {
                (
                    head.head.identity.clone(),
                    head.head.configuration.as_ref().map(|configuration| {
                        SavedConfiguration::from_config(&configuration, &state.id_to_mode)
                    }),
                )
            })
            .collect::<HashMap<_, _>>();
        let layout_match = state
            .layout_data
            .find_layout_match(&(current_layout.keys().cloned().collect()));
        match (
            layout_match,
            // If save_and_exit is set, then we don't want to apply the layout at all.
            if state.args.save_and_exit {
                DoneAction::Update
            } else {
                state.done_action
            },
        ) {
            (None, DoneAction::Update | DoneAction::Apply) => {
                info!(
                    "Saved layout: {:?}",
                    current_layout
                        .keys()
                        .map(|head_identity| head_identity.description.as_str())
                        .collect::<HashSet<_>>()
                );
                state.layout_data.layouts.push(current_layout);
                state.save_layouts();
                if state.args.save_and_exit {
                    // Bail out after the save.
                    std::process::exit(0);
                }
                // Ensure we go back to updating.
                state.done_action = DoneAction::Update;
            }
            (None, DoneAction::ApplyResult) => {
                panic!("We applied a layout, but then that layout didn't match?");
            }
            (Some((layout_index, _)), DoneAction::Update) => {
                info!(
                    "Update layout: {:?}",
                    current_layout
                        .keys()
                        .map(|head_identity| head_identity.description.as_str())
                        .collect::<HashSet<_>>()
                );
                state.layout_data.layouts[layout_index] = current_layout;
                state.save_layouts();
                if state.args.save_and_exit {
                    // Bail out after the save.
                    std::process::exit(0);
                }
            }
            (Some((layout_index, layout_head_to_query_head)), DoneAction::Apply) => {
                info!(
                    "Apply layout: {:?}",
                    state.layout_data.layouts[layout_index]
                        .keys()
                        .map(|head_identity| head_identity.description.as_str())
                        .collect::<HashSet<_>>()
                );
                state.apply_layout(
                    layout_index,
                    layout_head_to_query_head,
                    proxy,
                    qhandle,
                    serial,
                );
            }
            (Some(_), DoneAction::ApplyResult) => {
                debug!("Ignored the Done event since this is the result of an Apply");
            }
        }
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
        let partial_head = &mut state
            .partial_objects
            .id_to_head
            .entry(proxy.id())
            .or_insert_with(|| PartialHeadState {
                proxy: proxy.clone(),
                head: PartialHead::default(),
            })
            .head;
        debug!("Received Head event for head={:?}: {event:?}", proxy.id());
        match event {
            zwlr_output_head_v1::Event::Finished => {
                state.partial_objects.id_to_head.remove(&proxy.id());
                if let Some(head) = state.id_to_head.remove(&proxy.id()) {
                    assert!(
                        state
                            .head_identity_to_id
                            .remove(&head.head.identity)
                            .is_some(),
                        "Missing HeadIdentity for existing head"
                    );
                }
                proxy.release();
                // This head was removed, so try to apply a layout on the next `Done` event.
                state.done_action = DoneAction::Apply;
            }
            zwlr_output_head_v1::Event::Name { name } => {
                partial_head.name = Some(name);
            }
            zwlr_output_head_v1::Event::Description { description } => {
                partial_head.description = Some(description);
            }
            zwlr_output_head_v1::Event::Make { make } => {
                partial_head.make = Some(make);
            }
            zwlr_output_head_v1::Event::Model { model } => {
                partial_head.model = Some(model);
            }
            zwlr_output_head_v1::Event::SerialNumber { serial_number } => {
                partial_head.serial_number = Some(serial_number);
            }
            zwlr_output_head_v1::Event::Mode { mode } => {
                partial_head.modes.push(mode.id());
                state.partial_objects.id_to_mode.insert(
                    mode.id(),
                    PartialModeState {
                        proxy: mode,
                        mode: Default::default(),
                    },
                );
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                partial_head.enabled = Some(enabled > 0);
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => {
                partial_head.current_mode = Some(mode.id());
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                partial_head.position = Some((x as u32, y as u32));
            }
            zwlr_output_head_v1::Event::Transform { transform } => {
                let transform = transform
                    .into_result()
                    .expect("Transform is an invalid variant");
                let transform = transform.try_into().expect("Transform does not match");
                partial_head.transform = Some(transform);
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                partial_head.scale = Some(scale);
            }
            zwlr_output_head_v1::Event::AdaptiveSync { state } => {
                let state = state
                    .into_result()
                    .expect("Adaptive sync is an invalid variant");
                let state = match state {
                    AdaptiveSyncState::Enabled => Some(true),
                    AdaptiveSyncState::Disabled => Some(false),
                    _ => None,
                };
                partial_head.adaptive_sync = state;
            }
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
        debug!("Received Mode event for mode={:?}: {event:?}", proxy.id());
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                let partial_mode = state
                    .partial_objects
                    .id_to_mode
                    .get_mut(&id)
                    .expect("The mode was previously reported and not finished.");
                partial_mode.mode.size = Some((width as u32, height as u32));
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                let partial_mode = state
                    .partial_objects
                    .id_to_mode
                    .get_mut(&id)
                    .expect("The mode was previously reported and not finished.");
                partial_mode.mode.refresh = Some(refresh as u32);
            }
            zwlr_output_mode_v1::Event::Finished => {
                state.partial_objects.id_to_mode.remove(&id);
                state.id_to_mode.remove(&id);
                // Go through each head and remove any modes that use the id.
                for head in state.id_to_head.values_mut() {
                    head.head
                        .mode_to_id
                        .retain(|_, mode_in_head_id| *mode_in_head_id != id);
                }
                proxy.release();
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        debug!(
            "Received Configuration event for config={:?}: {event:?}",
            proxy.id()
        );
        match event {
            zwlr_output_configuration_v1::Event::Succeeded => {
                // We've applied the configuration! We can now get back to updating.
                state.done_action = DoneAction::Update;
                if let Some(apply_command) = state.args.apply_command.clone() {
                    run_command(apply_command);
                }
            }
            zwlr_output_configuration_v1::Event::Cancelled => {
                // Try to apply the layout again.
                state.done_action = DoneAction::Apply;
            }
            zwlr_output_configuration_v1::Event::Failed => {
                eprintln!("Failed to apply output configuration");
                // Try to apply the layout again.
                state.done_action = DoneAction::Apply;
            }
            _ => {}
        }
        proxy.destroy();
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        _event: zwlr_output_configuration_head_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // There are no events here.
    }
}

fn run_command(command: Arc<str>) {
    std::thread::spawn(
        move || match Command::new("sh").arg("-c").arg(command.as_ref()).output() {
            Ok(output) => {
                if output.status.success() {
                    debug!(
                        "post_exec command output:\nstdout={}\nstderr={}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr),
                    );
                } else {
                    error!(
                        "post_exec command failed with output:\nstdout={}\nstderr={}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr),
                    );
                }
            }
            Err(err) => {
                error!("Failed to run post_exec command: {err}");
            }
        },
    );
}
