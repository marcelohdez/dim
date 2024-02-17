use anyhow::{Context, Ok};
use dim::{dim::DimData, INIT_SIZE};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::{globals::registry_queue_init, Connection},
    registry::SimpleGlobal,
    shell::{
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell},
        WaylandSurface,
    },
};
use wayland_protocols::wp::{
    single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1,
    viewporter::client::wp_viewporter,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let single_pixel_mngr = SimpleGlobal::<WpSinglePixelBufferManagerV1, 1>::bind(&globals, &qh)
        .context("wp_single_pixel_buffer_manager_v1 not available!")?;
    let buffer = single_pixel_mngr
        .get()
        .context("Failed to get buffer manager")?
        .create_u32_rgba_buffer(0, 0, 0, u32::MAX, &qh, ());

    let surface = compositor.create_surface(&qh);

    let wp_viewporter = SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(&globals, &qh)
        .expect("wp_viewporter not available");
    let viewport = wp_viewporter
        .get()
        .expect("wp_viewporter failed")
        .get_viewport(&surface, &qh, ());

    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;
    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("dim_layer"), None);

    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.set_size(INIT_SIZE, INIT_SIZE);

    layer.commit();

    let mut data = DimData::new(&globals, &qh, buffer, viewport, layer);

    loop {
        event_queue
            .blocking_dispatch(&mut data)
            .context("Failed to block on events!")?;

        if data.should_exit() {
            break;
        }
    }

    Ok(())
}
