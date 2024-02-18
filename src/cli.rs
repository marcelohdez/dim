use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(short, long, help = "Duration in seconds")]
    pub duration: Option<u64>,

    #[arg(
        short,
        long,
        help = "0.0 is transparent, 1.0 is opaque, default is 0.5"
    )]
    pub alpha: Option<f32>,
}
