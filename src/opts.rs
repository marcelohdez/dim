use std::{
    borrow::Cow,
    env,
    ops::Deref,
    path::{Path, PathBuf},
};

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate_to, Shell};

/// Default duration in seconds
pub const DEFAULT_DURATION: u64 = 30;

pub const DEFAULT_ALPHA: f32 = 0.5;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(
        short,
        long,
        help = format!("Duration in seconds, 0 is infinite, default is {DEFAULT_DURATION}")
    )]
    pub duration: Option<u64>,

    #[arg(
        short,
        long,
        help = format!("0.0 is transparent, 1.0 is opaque, default is {DEFAULT_ALPHA}")
    )]
    pub alpha: Option<f32>,

    #[arg(long, value_name = "PATH", help = "Generate completions at given path")]
    pub gen_completions: Option<PathBuf>,
}

impl DimOpts {
    pub fn generate_completions(at: Option<&Path>) -> anyhow::Result<()> {
        let mut cli = Self::command();

        let dir = match at {
            Some(dir) => Cow::Borrowed(dir),
            None => Cow::Owned(env::current_dir()?),
        };

        for &shell in Shell::value_variants() {
            let comp_file = generate_to(shell, &mut cli, "dim", dir.deref())?;
            println!("cargo:warning=generated completion for {shell}: {comp_file:?}");
        }

        Ok(())
    }
}
