use std::{process, thread, time::Duration};

use anyhow::{anyhow, Context};
use clap::Parser;
use dim::{cli::DimOpts, dim::DimData};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::{globals::registry_queue_init, Connection},
    shell::wlr_layer::LayerShell,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = DimOpts::parse();

    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;
    let mut data = DimData::new(compositor, &globals, &qh, layer_shell);

    let duration = args.duration.unwrap_or(30);
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(duration));
        process::exit(0);
    });

    while !data.should_exit() {
        event_queue
            .blocking_dispatch(&mut data)
            .context("Failed to block on events!")?;
    }

    Err(anyhow!("Some user input was detected!"))
}
