use clap::Parser;

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
}
