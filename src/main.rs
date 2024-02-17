use anyhow::{Context, Ok};
use dim::{dim::DimData, INIT_SIZE};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::{
        client::{globals::registry_queue_init, Connection},
        protocols::wp::viewporter::client::wp_viewporter,
    },
    registry::SimpleGlobal,
    shell::{
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell},
        WaylandSurface,
    },
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let surface = compositor.create_surface(&qh);

    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;
    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("dim_layer"), None);

    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.set_size(INIT_SIZE, INIT_SIZE);

    layer.commit();

    let viewport = SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(&globals, &qh)
        .expect("wp_viewporter not available")
        .get()
        .expect("wp_viewporter failed")
        .get_viewport(layer.wl_surface(), &qh, ());

    let mut data = DimData::new(&globals, &qh, viewport, layer);

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
