use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate_to, Shell};
use serde::Deserialize;

use crate::{consts::DEFAULT_ALPHA, consts::DEFAULT_DURATION};

#[derive(Debug, Deserialize, Parser)]
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
        help = format!("0.0 is transparent, 1.0 is opaque. When opaque, cursor will be hidden. [default: {DEFAULT_ALPHA}]")
    )]
    pub alpha: Option<f32>,

    #[arg(
        short,
        long,
        help = "Make dim ignore input, passing it to lower surfaces. (You probably want to use `-d 0` with this)"
    )]
    #[serde(default)]
    pub passthrough: bool,

    #[serde(skip)]
    #[arg(long, value_name = "PATH", help = "Generate completions at given path")]
    pub gen_completions: Option<PathBuf>,

    #[serde(skip)]
    #[arg(short, long, value_name = "PATH", help = "Use config at path")]
    pub config: Option<PathBuf>,
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

    /// Merge other onto self, with other's values taking precedent
    pub fn merge_onto_self(self, other: DimOpts) -> Self {
        Self {
            duration: other.duration.or(self.duration),
            alpha: other.alpha.or(self.alpha),
            passthrough: self.passthrough || other.passthrough,

            ..self
        }
    }

    /// Validate that the received values are within our limits, should be called before using this
    /// object.
    pub fn validate(&self) -> Result<()> {
        if let Some(alpha) = self.alpha {
            if !(0.0..=1.0).contains(&alpha) {
                return Err(anyhow!("Alpha can only be from 0.0 to 1.0 inclusive."));
            }
        }

        Ok(())
    }

    /// Get user desired alpha or the default value.
    pub fn alpha(&self) -> f32 {
        self.alpha.unwrap_or(DEFAULT_ALPHA)
    }

    /// Get user desired duration or the default value.
    pub fn duration(&self) -> u64 {
        self.duration.unwrap_or(DEFAULT_DURATION)
    }
}
