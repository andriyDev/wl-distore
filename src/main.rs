use wayland_client::{event_created_child, protocol::wl_registry, Connection, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1, zwlr_output_manager_v1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

fn main() {
    let connection = Connection::connect_to_env().expect("Failed to establish a connection");
    let display = connection.display();

    let mut event_queue = connection.new_event_queue();
    let qhandle = event_queue.handle();

    display.get_registry(&qhandle, ());

    let mut app_data = AppData;
    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

struct AppData;

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        _state: &mut Self,
        proxy: &wl_registry::WlRegistry,
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

impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let zwlr_output_manager_v1::Event::Done { .. } = event else {
            return;
        };
        // TODO: Notify that the outputs are done.
    }

    event_created_child!(AppData, zwlr_output_head_v1::ZwlrOutputHeadV1, [
       zwlr_output_manager_v1::EVT_HEAD_OPCODE => (zwlr_output_head_v1::ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        dbg!(event);
    }

    event_created_child!(AppData, ZwlrOutputModeV1, [
        zwlr_output_head_v1::EVT_CURRENT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
        zwlr_output_head_v1::EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputModeV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputModeV1,
        _event: <ZwlrOutputModeV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
