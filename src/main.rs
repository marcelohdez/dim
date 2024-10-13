use std::{borrow::Cow, fs::File, io::read_to_string, path::Path, process, thread, time::Duration};

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
    args.validate()?;

    if let Some(path) = args.gen_completions {
        DimOpts::generate_completions(&path)?;
        return Ok(());
    }

    let opts = match get_config(args.config.as_deref()).context("Failed to read config!")? {
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

fn get_config(dir: Option<&Path>) -> anyhow::Result<Option<DimOpts>> {
    let project_dirs = ProjectDirs::from("com", "marcelohdez", "dim");

    let Some(dir): Option<Cow<Path>> = dir
        .map(Cow::Borrowed)
        .or(project_dirs.map(|dirs| Cow::Owned(dirs.config_dir().join(CONFIG_FILENAME))))
    else {
        bail!("Could not generate project directories on this OS?");
    };

    if !dir.exists() {
        info!("No config found!");
        return Ok(None);
    }

    debug!("Config file found at {dir:?}");
    let file = File::open(dir).context("Failed to open config file")?;
    let config: DimOpts = toml::from_str(&read_to_string(file)?)?;

    debug!("Config: {config:?}");
    config.validate()?;
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
