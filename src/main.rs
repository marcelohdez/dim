use std::{fs::File, io::read_to_string, process, thread, time::Duration};

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use dim_screen::{dim::DimData, opts::DimOpts, CONFIG_FILENAME, DEFAULT_ALPHA, DEFAULT_DURATION};
use directories::ProjectDirs;
use log::{debug, info};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::{globals::registry_queue_init, Connection},
    shell::wlr_layer::LayerShell,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = DimOpts::parse();

    if let Some(path) = args.gen_completions {
        DimOpts::generate_completions(&path)?;
        return Ok(());
    }

    let Some(dirs) = ProjectDirs::from("com", "marcelohdez", "dim") else {
        bail!("Could not generate project directories on this OS?");
    };
    let config_dir = dirs.config_dir().join(CONFIG_FILENAME);

    let opts = if config_dir.exists() {
        debug!("Config file found at {config_dir:?}");

        let file = File::open(config_dir)?;
        let config: DimOpts = toml::from_str(&read_to_string(file)?)?;

        debug!("Config: {config:?}");
        config.merge_onto_self(args)
    } else {
        info!("Config file not found!");
        args
    };

    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;

    debug!("Using options: {opts:?}");
    let alpha = opts.alpha.unwrap_or(DEFAULT_ALPHA);
    let duration = opts.duration.unwrap_or(DEFAULT_DURATION);

    // A duration of 0 is considered as infinite, not starting the timer:
    if duration > 0 {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(duration));
            process::exit(0);
        });
    }

    let mut data = DimData::new(compositor, &globals, &qh, layer_shell, alpha);
    while !data.should_exit() {
        event_queue
            .blocking_dispatch(&mut data)
            .context("Failed to block on events!")?;
    }

    Err(anyhow!("Some user input was detected!"))
}
