use std::{fs::File, io::read_to_string, process, thread, time::Duration};

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use dim_screen::{dim::DimData, opts::DimOpts, CONFIG_FILENAME, DEFAULT_ALPHA, DEFAULT_DURATION};
use directories::ProjectDirs;
use log::{debug, info};
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::{globals::registry_queue_init, Connection, EventQueue},
    shell::wlr_layer::LayerShell,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = DimOpts::parse();

    if let Some(path) = args.gen_completions {
        DimOpts::generate_completions(&path)?;
        return Ok(());
    }

    let opts = match get_config()? {
        Some(config) => config.merge_onto_self(args),
        None => args,
    };

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

    let (mut data, mut event_queue) = create_wl_app(alpha)?;
    while !data.should_exit() {
        event_queue
            .blocking_dispatch(&mut data)
            .context("Failed to block on events!")?;
    }

    Err(anyhow!("Some user input was detected!"))
}

fn get_config() -> anyhow::Result<Option<DimOpts>> {
    let Some(dirs) = ProjectDirs::from("com", "marcelohdez", "dim") else {
        bail!("Could not generate project directories on this OS?");
    };
    let config_dir = dirs.config_dir().join(CONFIG_FILENAME);

    if !config_dir.exists() {
        info!("No config found!");
        return Ok(None);
    }

    debug!("Config file found at {config_dir:?}");

    let file = File::open(config_dir)?;
    let config: DimOpts = toml::from_str(&read_to_string(file)?)?;

    debug!("Config: {config:?}");
    Ok(Some(config))
}

fn create_wl_app(alpha: f32) -> anyhow::Result<(DimData, EventQueue<DimData>)> {
    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;

    Ok((
        DimData::new(compositor, &globals, &qh, layer_shell, alpha),
        event_queue,
    ))
}
