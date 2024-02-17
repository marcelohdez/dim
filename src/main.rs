use anyhow::{Context, Ok};
use dim::dim::DimData;
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::{globals::registry_queue_init, Connection},
    shell::wlr_layer::LayerShell,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;
    let mut data = DimData::new(compositor, &globals, &qh, layer_shell);

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
