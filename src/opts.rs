use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate_to, Shell};

const DEFAULT_DURATION: u64 = 30;
const DEFAULT_ALPHA: f32 = 0.5;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(
        short,
        long,
        default_value_t = DEFAULT_DURATION,
        help = "Duration in seconds, 0 is infinite"
    )]
    pub duration: u64,

    #[arg(
        short,
        long,
        default_value_t = DEFAULT_ALPHA,
        help = "0.0 is transparent, 1.0 is opaque"
    )]
    pub alpha: f32,

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
