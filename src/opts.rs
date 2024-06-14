use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate_to, Shell};

use crate::{DEFAULT_ALPHA, DEFAULT_DURATION};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(
        short,
        long,
        help = format!("Duration in seconds, 0 is infinite, [default: {DEFAULT_DURATION}]")
    )]
    pub duration: Option<u64>,

    #[arg(
        short,
        long,
        help = format!("0.0 is transparent, 1.0 is opaque, [default: {DEFAULT_ALPHA}]")
    )]
    pub alpha: Option<f32>,

    #[arg(long, value_name = "PATH", help = "Generate completions at given path")]
    pub gen_completions: Option<PathBuf>,
}

impl DimOpts {
    pub fn generate_completions(dir: &Path) -> anyhow::Result<()> {
        let mut cli = Self::command();

        for &shell in Shell::value_variants() {
            let comp_file = generate_to(shell, &mut cli, "dim", dir)?;
            println!("Generated completion for {shell} at {comp_file:?}");
        }

        Ok(())
    }
}
