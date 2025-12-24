use std::{
    borrow::Cow,
    env,
    fs::File,
    io::read_to_string,
    path::{Path, PathBuf},
    process, thread,
    time::Duration,
};

use anyhow::{anyhow, Context};
use clap::Parser;
use dim_screen::{consts::CONFIG_FILENAME, DimData, DimOpts};
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
        return DimOpts::generate_completions(&path);
    }

    let opts = match get_config(args.config.as_deref()).context("Failed to read config!")? {
        Some(config) => config.merge_onto_self(args),
        None => args,
    };

    debug!("Using options: {opts:?}");

    let duration = opts.duration();
    // We consider a duration of 0 as infinite, not starting the timer
    if duration > 0 {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(duration));
            process::exit(0);
        });
    }

    let (mut data, mut event_queue) = create_wl_app(opts)?;
    while !data.should_exit() {
        event_queue
            .blocking_dispatch(&mut data)
            .context("Failed to block on events!")?;
    }

    Err(anyhow!("Some user input was detected!"))
}

fn get_config(dir: Option<&Path>) -> anyhow::Result<Option<DimOpts>> {
    let config = match dir {
        Some(user_config) => Cow::Borrowed(user_config),
        None => {
            // follow XDG base directory spec, checking $XDG_CONFIG_HOME first then defaulting to $HOME/.config
            let config_home = env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or(env::var("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".config")));

            if let Ok(path) = config_home {
                Cow::Owned(path.join("dim").join(CONFIG_FILENAME))
            } else {
                info!("No config path, neither XDG_CONFIG_HOME nor HOME are set.");
                return Ok(None);
            }
        }
    };

    if !config.exists() {
        info!("No config found!");
        return Ok(None);
    }

    debug!("Config file found at {config:?}");
    let file = File::open(config).context("Failed to open config file")?;
    let config: DimOpts = toml::from_str(&read_to_string(file)?)?;

    debug!("Config: {config:?}");
    config.validate()?;
    Ok(Some(config))
}

fn create_wl_app(opts: DimOpts) -> anyhow::Result<(DimData, EventQueue<DimData>)> {
    let conn = Connection::connect_to_env().context("Failed to connect to environment")?;

    let (globals, event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell failed?")?;

    Ok((
        DimData::new(compositor, &globals, &qh, layer_shell, opts),
        event_queue,
    ))
}
