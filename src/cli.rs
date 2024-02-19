use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(
        short,
        long,
        help = "Duration in seconds, 0 is infinite, default is 30"
    )]
    pub duration: Option<u64>,

    #[arg(
        short,
        long,
        help = "0.0 is transparent, 1.0 is opaque, default is 0.5"
    )]
    pub alpha: Option<f32>,
}
